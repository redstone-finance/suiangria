"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.publishPackage = publishPackage;
const child_process_1 = require("child_process");
const fs_1 = require("fs");
function publishPackage(sandbox, movePackageSrcDir, owner) {
    const sui = findSuiBinary();
    const buildOutput = compileMovePackage(sui, movePackageSrcDir);
    const moduleBytes = extractModuleBytes(buildOutput);
    const dependencyIds = extractDependencyIds(buildOutput);
    return sandbox.publishPackage(moduleBytes, dependencyIds, owner);
}
function compileMovePackage(suiPath, packageDir) {
    const buildCommand = `${suiPath} move build --path ${packageDir} --dump-bytecode-as-base64 --skip-fetch-latest-git-deps`;
    try {
        const output = (0, child_process_1.execSync)(buildCommand, {
            encoding: 'utf8',
            env: { ...process.env, PATH: process.env.PATH },
        });
        return parseBuildOutput(output);
    }
    catch (error) {
        console.error(error);
        throw new Error(`Failed to build Move package: ${error}`);
    }
}
function findSuiBinary() {
    try {
        const path = (0, child_process_1.execSync)('which sui', { encoding: 'utf8' }).trim();
        if (path)
            return path;
    }
    catch { }
    const possiblePaths = [
        '/usr/local/bin/sui',
        `${process.env.HOME}/.cargo/bin/sui`,
        `${process.env.HOME}/.local/bin/sui`,
    ];
    for (const path of possiblePaths) {
        if ((0, fs_1.existsSync)(path))
            return path;
    }
    return 'sui';
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