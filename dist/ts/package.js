'use strict';
Object.defineProperty(exports, '__esModule', { value: true });
exports.publishPackage = publishPackage;
const child_process_1 = require('child_process');
function publishPackage(sandbox, movePackageSrcDir, owner) {
  const buildOutput = compileMovePackage(movePackageSrcDir);
  const moduleBytes = extractModuleBytes(buildOutput);
  const dependencyIds = extractDependencyIds(buildOutput);
  return sandbox.publishPackage(moduleBytes, dependencyIds, owner);
}
function compileMovePackage(packageDir) {
  const buildCommand = `sui move build --path ${packageDir} --dump-bytecode-as-base64 --skip-fetch-latest-git-deps`;
  try {
    const output = (0, child_process_1.execSync)(buildCommand, {
      encoding: 'utf8',
      env: { ...process.env, PATH: process.env.PATH },
      stdio: ['inherit', 'pipe', 'inherit'],
    });
    return parseBuildOutput(output);
  } catch (error) {
    throw new Error(`Failed to build Move package: ${error}`);
  }
}
function parseBuildOutput(output) {
  const jsonMatch = output.match(/\{[\s\S]*\}/);
  if (!jsonMatch) {
    throw new Error('Failed to parse build output - no JSON found');
  }
  const buildInfo = JSON.parse(jsonMatch[0]);
  return buildInfo;
}
function extractModuleBytes(buildOutput) {
  return buildOutput.modules.map((base64Module) => {
    const bytes = Buffer.from(base64Module, 'base64');
    return Array.from(bytes);
  });
}
function extractDependencyIds(buildOutput) {
  const suiFrameworkDeps = [
    '0x0000000000000000000000000000000000000000000000000000000000000001',
    '0x0000000000000000000000000000000000000000000000000000000000000002',
  ];
  return [...suiFrameworkDeps, ...buildOutput.dependencies.filter((dep) => !suiFrameworkDeps.includes(dep))];
}
//# sourceMappingURL=package.js.map
