import { spawn } from 'node:child_process'

const pnpmEntry = process.env.npm_execpath
const command = pnpmEntry ? process.execPath : 'pnpm'
const args = pnpmEntry ? [pnpmEntry, 'build'] : ['build']

const child = spawn(command, args, {
  stdio: 'inherit',
  env: {
    ...process.env,
    NODE_OPTIONS: '--max-old-space-size=4096'
  },
  shell: !pnpmEntry && process.platform === 'win32'
})

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }

  process.exit(code ?? 1)
})

child.on('error', (error) => {
  console.error('[build:ci] failed to start build process:', error)
  process.exit(1)
})
