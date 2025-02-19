use quick_protobuf::{deserialize_from_slice, serialize_into_vec};

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn as spawn;

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

use std::{collections::{HashMap, HashSet}, future::Future, sync::Arc};
use async_trait::async_trait;
use libp2p::Stream;
use networking::context::Context;
use protobuf::task::{self, mod_ResourceRecruitment as ResourceRecruitment, Status, Task};
use crate::{cluster::{DomainCluster, TaskUpdateEvent, TaskUpdateResult}, datastore::common::{DataReader, DataWriter, Datastore, DomainError}, protobuf::domain_data::{self, Data, Metadata}};
use futures::{channel::mpsc::channel, future::Remote, io::ReadHalf, lock::Mutex, AsyncReadExt, AsyncWriteExt, SinkExt, StreamExt};

use super::common::{Reader, ReliableDataProducer, Writer};

pub const CONSUME_DATA_PROTOCOL_V1: &str = "/consume/v1";
pub const PRODUCE_DATA_PROTOCOL_V1: &str = "/produce/v1";

struct TaskHandler {
    #[cfg(not(target_family = "wasm"))]
    handler: Option<tokio::task::JoinHandle<()>>,
    #[cfg(target_family = "wasm")]
    handler: Option<()>,
}

impl TaskHandler {
    fn new() -> Self {
        Self { handler: None }
    }
    // Define the function to execute the handler
    fn execute<F>(&mut self, handler: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        #[cfg(not(target_family = "wasm"))]
        {
            // Non-WASM environment: spawn using tokio
            self.handler = Some(spawn(handler));
        }

        #[cfg(target_family = "wasm")]
        {
            // WASM environment: spawn using spawn_local
            spawn(handler);
        }
    }

    fn cancel(&mut self) {
        #[cfg(not(target_family = "wasm"))]
        {
            if let Some(handler) = self.handler.take() {
                handler.abort();
            }
        }
    }
}

#[derive(Clone)]
pub struct RemoteDatastore {
    cluster: DomainCluster,
    peer: Context,
}

impl RemoteDatastore {
    pub fn new(cluster: DomainCluster, peer: Context ) -> Self {
        Self { cluster, peer }
    }

    // TODO: error handling
    async fn read_from_stream(domain_id: String, mut src: ReadHalf<Stream>, mut dest: DataWriter) {
        loop {
            let mut length_buf = [0u8; 4];
            let has = src.read(&mut length_buf).await.expect("Failed to read length");
            if has == 0 {
                break;
            }
    
            let length = u32::from_be_bytes(length_buf) as usize;
    
            // Read the data in chunks
            let mut buffer = vec![0u8; length];
            src.read_exact(&mut buffer).await.expect("Failed to read buffer");
    
            let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer).expect("Failed to deserialize metadata");
    
            let mut buffer = vec![0u8; metadata.size as usize];
            src.read_exact(&mut buffer).await.expect("Failed to read buffer");

            let data = Data {
                metadata,
                domain_id: domain_id.clone(),
                content: buffer,
            };
            let _ = dest.send(Ok(data));
        }
    }

    async fn write_to_stream(src: Arc<Mutex<DataReader>>, stream: Stream, mut response_sender: Writer<Metadata>) {
        let mut src = src.lock().await;

        let (mut reader, mut writer) = stream.split();
        spawn(async move {
            loop {
                let mut length_buf = [0u8; 4];
                let has = reader.read(&mut length_buf).await.expect("Failed to read length");
                if has == 0 {
                    break;
                }
        
                let length = u32::from_be_bytes(length_buf) as usize;
        
                // Read the data in chunks
                let mut buffer = vec![0u8; length];
                reader.read_exact(&mut buffer).await.expect("Failed to read buffer");
        
                let metadata = deserialize_from_slice::<domain_data::Metadata>(&buffer).expect("Failed to deserialize metadata");
                match response_sender.send(Ok(metadata)).await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("{}", e);
                        break;
                    }
                }
            }
        });
        while let Some(data) = src.next().await {
            match data {
                Ok(data) => {
                    let m_buf = serialize_into_vec(&data.metadata).expect("Failed to serialize metadata");
                    let mut length_buf = [0u8; 4];
                    let length = m_buf.len() as u32;
                    length_buf.copy_from_slice(&length.to_be_bytes());

                    println!("Uploading data {}, {}/{}", data.metadata.name, data.metadata.size, data.content.len());
                    assert_eq!(data.metadata.size as usize, data.content.len(), "size should match");
                    
                    writer.write_all(&length_buf).await.expect("Failed to write length");
                    writer.write_all(&m_buf).await.expect("Failed to write metadata");
                    writer.write_all(&data.content).await.expect("Failed to write content");
                    writer.flush().await.expect("Failed to flush");
                },
                Err(e) => {
                    eprintln!("Failed to read data: {:?}", e);
                    return;
                }
            }
        }
        writer.close().await.expect("Failed to close writer");
    }
}

#[async_trait]
impl Datastore for RemoteDatastore {
    async fn consume(&mut self, domain_id: String, query: domain_data::Query) -> DataReader
    {
        let mut download_task = TaskHandler::new();
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(100);
        let peer_id = self.peer.id.clone();
        let mut peer = self.peer.clone();
        let domain_id = domain_id.clone();
        let query = query.clone();
        let job = &task::Job {
            name: "stream download domain data".to_string(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: Some(task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    }),
                    name: "download_data".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: 1000,
                    capability_filters: Some(task::CapabilityFilters {
                        endpoint: "/consume/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    // TODO: task::Any type_url
                    data: Some(task::Any {
                        type_url: "type.googleapis.com/posemesh.task.DownloadData".to_string(),
                        value: serialize_into_vec(&query).unwrap(),
                    }),
                    sender: peer_id.clone(),
                    receiver: "".to_string(),
                }
            ],
        };

        let mut download_task_recv = self.cluster.submit_job(job).await;

        loop {
            let update = download_task_recv.next().await;
            match update {
                Some(TaskUpdateEvent {
                    result: TaskUpdateResult::Ok(mut task),
                    ..
                }) => match task.status {
                    Status::PENDING => {
                        task.status = Status::STARTED;
                        let domain_id_clone = domain_id.clone();
                        let data_sender_clone = data_sender.clone();
                        peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                        let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
                            access_token: task.access_token.clone(),
                        }).unwrap();
                        let mut length_buf = [0u8; 4];
                        let length = m_buf.len() as u32;
                        length_buf.copy_from_slice(&length.to_be_bytes());
                        let upload_stream = peer.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 1000).await.expect("cant send handshake");
                        let (reader, _) = upload_stream.split();
                        download_task.execute(async move {
                            RemoteDatastore::read_from_stream(domain_id_clone, reader, data_sender_clone).await;
                        });
                        break;
                    },
                    Status::FAILED => {
                        eprintln!("Failed to download data: {:?}", task);
                        // download_task.cancel();
                    },
                    _ => ()
                }
                None => {
                    println!("task update channel is closed");
                    break;
                }
                _ => ()
            }
        }

        spawn(async move {
            loop {
                let update = download_task_recv.next().await;
                match update {
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(task),
                        ..
                    }) => match task.status {
                        Status::FAILED => {
                            eprintln!("Failed to download data: {:?}", task);
                            download_task.cancel();
                        },
                        _ => ()
                    }
                    None => {
                        println!("task update channel is closed");
                        download_task.cancel();
                        return;
                    }
                    _ => ()
                }
            } 
        });
        
        data_receiver
    }

    async fn produce(&mut self, domain_id: String) -> ReliableDataProducer {
        let (data_sender, data_receiver) = channel::<Result<Data, DomainError>>(100);
        let mut upload_task_handler = TaskHandler::new();
        let (uploaded_data_sender, uploaded_data_receiver) = channel::<Result<Metadata, DomainError>>(100);
        let mut upload_job_recv = self.cluster.submit_job(&task::Job {
            name: "stream uploading recordings".to_string(),
            tasks: vec![
                task::TaskRequest {
                    needs: vec![],
                    resource_recruitment: Some(task::ResourceRecruitment {
                        recruitment_policy: ResourceRecruitment::RecruitmentPolicy::FAIL,
                        termination_policy: ResourceRecruitment::TerminationPolicy::KEEP,
                    }),
                    name: "store_recording".to_string(),
                    timeout: "100m".to_string(),
                    max_budget: 1000,
                    capability_filters: Some(task::CapabilityFilters {
                        endpoint: "/produce/v1".to_string(),
                        min_gpu: 0,
                        min_cpu: 0,
                    }),
                    data: None,
                    sender: self.peer.id.clone(),
                    receiver: "".to_string(),
                }
            ],
        }).await;

        let data_receiver = Arc::new(Mutex::new(data_receiver));
        loop {
            let update = upload_job_recv.next().await;
            match update {
                Some(TaskUpdateEvent {
                    result: TaskUpdateResult::Ok(mut task),
                    ..
                }) => match task.status {
                    Status::PENDING => {
                        task.status = Status::STARTED;
                        self.peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                        let m_buf = serialize_into_vec(&task::DomainClusterHandshake{
                            access_token: task.access_token.clone(),
                        }).unwrap();
                        let mut length_buf = [0u8; 4];
                        let length = m_buf.len() as u32;
                        length_buf.copy_from_slice(&length.to_be_bytes());
                        let mut upload_stream = self.peer.send(length_buf.to_vec(), task.receiver.clone(), task.endpoint.clone(), 1000).await.expect("cant send handshake");
                        upload_stream.write_all(&m_buf).await.expect("cant write handshake");
                        upload_stream.flush().await.expect("cant flush handshake");
                        let mut peer = self.peer.clone();
                        let handler = async move {
                            RemoteDatastore::write_to_stream(data_receiver, upload_stream, uploaded_data_sender).await;
                            let mut task = task.clone();
                            task.status = Status::DONE;
                            peer.publish(task.job_id.clone(), serialize_into_vec(&task).expect("Failed to serialize message")).await.expect("Failed to publish message");
                        };
                        upload_task_handler.execute(handler);
                        break;
                    },
                    Status::FAILED => {
                        // TODO: handle error
                        panic!("Failed to upload data: {:?}", task);
                    },
                    _ => ()
                }
                None => {
                    println!("task update channel is closed");
                    break;
                }
                _ => ()
            }
        }

        spawn(async move {
            loop {
                let update = upload_job_recv.next().await;
                match update {
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Ok(task),
                        ..
                    }) => match task.status {
                        Status::FAILED => {
                            eprintln!("Failed to upload data: {:?}", task);
                            upload_task_handler.cancel();
                        },
                        _ => ()
                    }
                    None => {
                        println!("task update channel is closed");
                        upload_task_handler.cancel();
                        return;
                    }
                    Some(TaskUpdateEvent {
                        result: TaskUpdateResult::Err(e),
                        ..
                    }) => {
                        eprintln!("Task update failure: {:?}", e);
                        upload_task_handler.cancel();
                        return;
                    }
                }
            } 
        });

        ReliableDataProducer::new(uploaded_data_receiver, data_sender)
    }
}
