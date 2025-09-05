import { execSync } from 'child_process'
import { SandboxClient } from './client'

interface BuildOutput {
  modules: string[]
  dependencies: string[]
  digest: string
}

export function publishPackage(sandbox: SandboxClient, movePackageSrcDir: string, owner: string) {
  const buildOutput = compileMovePackage(movePackageSrcDir)
  const moduleBytes = extractModuleBytes(buildOutput)
  const dependencyIds = extractDependencyIds(buildOutput)

  return sandbox.publishPackage(moduleBytes, dependencyIds, owner)
}

function compileMovePackage(packageDir: string): BuildOutput {
  const buildCommand = `sui move build --path ${packageDir} --dump-bytecode-as-base64 --skip-fetch-latest-git-deps`

  try {
    const output = execSync(buildCommand, {
      encoding: 'utf8',
      env: { ...process.env, PATH: process.env.PATH },
      stdio: 'inherit',
    })

    return parseBuildOutput(output)
  } catch (error) {
    throw new Error(`Failed to build Move package: ${error}`)
  }
}

function parseBuildOutput(output: string): BuildOutput {
  const jsonMatch = output.match(/\{[\s\S]*\}/)
  if (!jsonMatch) {
    throw new Error('Failed to parse build output - no JSON found')
  }

  const buildInfo = JSON.parse(jsonMatch[0])
  return buildInfo
}

function extractModuleBytes(buildOutput: BuildOutput): number[][] {
  return buildOutput.modules.map((base64Module) => {
    const bytes = Buffer.from(base64Module, 'base64')
    return Array.from(bytes)
  })
}

function extractDependencyIds(buildOutput: BuildOutput): string[] {
  const suiFrameworkDeps = [
    '0x0000000000000000000000000000000000000000000000000000000000000001',
    '0x0000000000000000000000000000000000000000000000000000000000000002',
  ]

  return [...suiFrameworkDeps, ...buildOutput.dependencies.filter((dep) => !suiFrameworkDeps.includes(dep))]
}
