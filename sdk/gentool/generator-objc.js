const fs = require('fs');
const path = require('path');
const util = require('./util');

function generateHeader(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.ObjC);
  const nameSwift = util.getLangClassName(interfaceJson, util.Swift);
  const copyable = util.getClassCopyable(interfaceJson);
  const copyableExt = copyable ? '<NSCopying>' : '';
  const static = util.getClassStatic(interfaceJson);
  const managedGetterName = `managed${interfaceName}`;
  const nativeGetterName = `native${interfaceName}`;

  let importsFirst = new Set(['#import <Foundation/Foundation.h>']), importsSecond = new Set(['#import "API.h"']);
  let includesFirst = new Set([]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  code += `NS_SWIFT_NAME(${nameSwift}) PSM_API @interface ${name} : NSObject${copyableExt}\n`;

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (static || pCtorDefinition === util.ConstructorDefinition.deleted || pCtorVisibility !== util.Visibility.public) {
    publicCtors += '- (instancetype)init NS_UNAVAILABLE;\n';
  } else {
    publicCtors += '- (instancetype)init;\n';
  }

  let public = publicCtors;
  if (publicOperators.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicOperators;
  }
  if (publicMethods.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicMethods;
  }
  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }

  if (public.length > 0) {
    code += '\n';
    code += public;
  }

  if (!static) {
    code += '\n';
    code += '#if defined(POSEMESH_BUILD)\n';
    code += `- (void*)${managedGetterName};\n`;
    code += `- (void*)${nativeGetterName};\n`;
    code += '#endif\n';
  }
  code += '\n';
  code += '@end\n';
  const aliases = util.getLangAliases(interfaceJson, util.ObjC);
  const aliasesSwift = util.getLangAliases(interfaceJson, util.Swift);
  const aliasesPaired = aliases.map((item, index) => [item, aliasesSwift[index]]);
  if (aliasesPaired.length > 0) {
    code += '\n';
    code += '#if defined(__swift__)\n';
    for (const aliasPaired of aliasesPaired) {
      code += `typedef ${name}* __${aliasPaired[0]} NS_SWIFT_NAME(${aliasPaired[1]});\n`;
    }
    code += '#else\n';
    for (const aliasPaired of aliasesPaired) {
      code += `@compatibility_alias ${aliasPaired[0]} ${name};\n`;
    }
    code += '#endif\n';
  }

  importsFirst = Array.from(importsFirst).sort();
  importsSecond = Array.from(importsSecond).sort();
  includesFirst = Array.from(includesFirst).sort();
  includesSecond = Array.from(includesSecond).sort();
  let includes = '';
  if (importsFirst.length > 0) {
    includes += '\n';
    for (const include of importsFirst) {
      includes += include + '\n';
    }
  }
  if (includesFirst.length > 0) {
    if (importsFirst.length > 0) {
      includes += '\n';
    }
    for (const include of includesFirst) {
      includes += include + '\n';
    }
  }
  if (importsSecond.length > 0) {
    includes += '\n';
    for (const include of importsSecond) {
      includes += include + '\n';
    }
  }
  if (includesSecond.length > 0) {
    if (importsSecond.length > 0) {
      includes += '\n';
    }
    for (const include of includesSecond) {
      includes += include + '\n';
    }
  }
  code = code.replaceAll('%INCLUDES%', includes);

  return code;
}

function generateSource(interfaceName, interfaceJson) {
  const name = util.getLangClassName(interfaceJson, util.ObjC);
  const nameCxx = util.getLangClassName(interfaceJson, util.CXX);
  const nameCamelBack = util.getStyleName('name', interfaceJson, util.camelBack);
  const nameManagedMember = `m_${nameCamelBack}`;
  const static = util.getClassStatic(interfaceJson);
  const managedGetterName = `managed${interfaceName}`;
  const nativeGetterName = `native${interfaceName}`;
  const initWithManagedName = `initWithManaged${interfaceName}`;
  const initWithNativeName = `initWithNative${interfaceName}`;

  let importsFirst = new Set([`#import <Posemesh/${interfaceName}.h>`]), importsSecond = new Set([]);
  let includesFirst = new Set([`#include <Posemesh/${interfaceName}.hpp>`]), includesSecond = new Set([]);

  let code = `/* This code is automatically generated from ${interfaceName}.json interface. Do not modify it manually as it will be overwritten! */\n`;
  code += '%INCLUDES%\n';
  if (static) {
    code += `@implementation ${name}\n`;
  } else {
    code += `@implementation ${name} {\n`;
    code += `    std::shared_ptr<psm::${nameCxx}> ${nameManagedMember};\n`;
    code += '}\n';
    includesFirst.add('#include <managed>');
  }

  let publicCtors = '', publicOperators = '', publicMethods = '', publicFuncs = '';
  let privateCtors = '', privateOperators = '', privateMethods = '', privateFuncs = '';

  const parameterlessConstructor = util.getClassParameterlessConstructor(interfaceJson);
  const pCtorDefinition = util.getConstructorDefinition(parameterlessConstructor);
  const pCtorVisibility = util.getConstructorVisibility(parameterlessConstructor);
  if (!static) {
    if (pCtorDefinition !== util.ConstructorDefinition.deleted) {
      let pCtor = '- (instancetype)init\n';
      pCtor += '{\n';
      pCtor += `    auto* ${nameCamelBack} = new (std::nothrow) psm::${nameCxx};\n`;
      pCtor += `    if (!${nameCamelBack}) {\n`;
      pCtor += `        return nil;\n`;
      pCtor += `    }\n`;
      pCtor += `    self = [self ${initWithNativeName}:${nameCamelBack}];\n`;
      pCtor += `    if (!self) {\n`;
      pCtor += `        delete ${nameCamelBack};\n`;
      pCtor += `        return nil;\n`;
      pCtor += `    }\n`;
      pCtor += `    return self;\n`;
      pCtor += '}\n';
      if (pCtorVisibility === util.Visibility.public) {
        if (publicCtors.length > 0) {
          publicCtors += '\n';
        }
        publicCtors += pCtor;
      } else {
        if (privateCtors.length > 0) {
          privateCtors += '\n';
        }
        privateCtors += pCtor;
      }

      includesFirst.add('#include <new>');
    }

    let initWithManaged = `- (instancetype)${initWithManagedName}:(std::shared_ptr<psm::${nameCxx}>)${nameCamelBack}\n`;
    initWithManaged += '{\n';
    initWithManaged += `    NSAssert(${nameCamelBack}.get() != nullptr, @"${nameCamelBack} is null");\n`;
    initWithManaged += `    self = [super init];\n`;
    initWithManaged += `    if (!self) {\n`;
    initWithManaged += `        return nil;\n`;
    initWithManaged += `    }\n`;
    initWithManaged += `    ${nameManagedMember} = std::move(${nameCamelBack});\n`;
    initWithManaged += `    return self;\n`;
    initWithManaged += '}\n';
    if (privateCtors.length > 0) {
      privateCtors += '\n';
    }
    privateCtors += initWithManaged;

    includesFirst.add('#include <utility>');

    let initWithNative = `- (instancetype)${initWithNativeName}:(psm::${nameCxx}*)${nameCamelBack}\n`;
    initWithNative += '{\n';
    initWithNative += `    NSAssert(${nameCamelBack} != nullptr, @"${nameCamelBack} is null");\n`;
    initWithNative += `    self = [super init];\n`;
    initWithNative += `    if (!self) {\n`;
    initWithNative += `        return nil;\n`;
    initWithNative += `    }\n`;
    initWithNative += `    try {\n`;
    initWithNative += `        ${nameManagedMember}.reset(${nameCamelBack});\n`;
    initWithNative += `    } catch (...) {\n`;
    initWithNative += `        return nil;\n`;
    initWithNative += `    }\n`;
    initWithNative += `    return self;\n`;
    initWithNative += '}\n';
    if (privateCtors.length > 0) {
      privateCtors += '\n';
    }
    privateCtors += initWithNative;
  }

  let public = publicCtors;
  if (publicOperators.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicOperators;
  }
  if (publicMethods.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicMethods;
  }
  if (publicFuncs.length > 0) {
    if (public.length > 0) {
      public += '\n';
    }
    public += publicFuncs;
  }

  let private = privateCtors;
  if (privateOperators.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateOperators;
  }
  if (privateMethods.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateMethods;
  }
  if (privateFuncs.length > 0) {
    if (private.length > 0) {
      private += '\n';
    }
    private += privateFuncs;
  }

  if (public.length > 0) {
    code += '\n';
    code += public;
  }
  if (private.length > 0) {
    code += '\n';
    code += private;
  }

  if (!static) {
    code += '\n';
    code += `- (void*)${managedGetterName}\n`;
    code += `{\n`;
    code += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    code += `    return &${nameManagedMember};\n`;
    code += `}\n`;
    code += '\n';
    code += `- (void*)${nativeGetterName}\n`;
    code += `{\n`;
    code += `    NSAssert(${nameManagedMember}.get() != nullptr, @"${nameManagedMember} is null");\n`;
    code += `    return ${nameManagedMember}.get();\n`;
    code += `}\n`;
  }
  code += '\n';
  code += '@end\n';

  importsFirst = Array.from(importsFirst).sort();
  importsSecond = Array.from(importsSecond).sort();
  includesFirst = Array.from(includesFirst).sort();
  includesSecond = Array.from(includesSecond).sort();
  let includes = '';
  if (importsFirst.length > 0) {
    includes += '\n';
    for (const include of importsFirst) {
      includes += include + '\n';
    }
  }
  if (includesFirst.length > 0) {
    if (importsFirst.length > 0) {
      includes += '\n';
    }
    for (const include of includesFirst) {
      includes += include + '\n';
    }
  }
  if (importsSecond.length > 0) {
    includes += '\n';
    for (const include of importsSecond) {
      includes += include + '\n';
    }
  }
  if (includesSecond.length > 0) {
    if (importsSecond.length > 0) {
      includes += '\n';
    }
    for (const include of includesSecond) {
      includes += include + '\n';
    }
  }
  code = code.replaceAll('%INCLUDES%', includes);

  return code;
}

function generateInterfaceObjC(interfaceName, interfaceJson) {
  const headerFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'include', 'Posemesh', `${interfaceName}.h`);
  const sourceFilePath = path.resolve(__dirname, '..', 'platform', 'Apple', 'src', `${interfaceName}.mm`);

  let headerCode = generateHeader(interfaceName, interfaceJson);
  let sourceCode = generateSource(interfaceName, interfaceJson);

  fs.writeFileSync(headerFilePath, headerCode, 'utf8');
  fs.writeFileSync(sourceFilePath, sourceCode, 'utf8');
}

module.exports = generateInterfaceObjC;
