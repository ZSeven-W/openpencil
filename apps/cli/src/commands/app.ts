import { getAppInfo } from '../connection'
import { startDesktop, startWeb, stopApp } from '../launcher'
import { output, outputError } from '../output'

export async function cmdStart(flags: {
  desktop?: boolean
  web?: boolean
}): Promise<void> {
  try {
    let result: { port: number; pid: number }
    if (flags.web) {
      result = await startWeb()
    } else {
      result = await startDesktop()
    }
    output({ ok: true, ...result, url: `http://127.0.0.1:${result.port}` })
  } catch (err) {
    outputError((err as Error).message)
  }
}

export async function cmdStop(): Promise<void> {
  const stopped = await stopApp()
  if (stopped) {
    output({ ok: true, message: 'OpenPencil stopped' })
  } else {
    output({ ok: true, message: 'No running instance found' })
  }
}

export async function cmdStatus(): Promise<void> {
  const info = await getAppInfo()
  if (info) {
    output({
      running: true,
      port: info.port,
      pid: info.pid,
      url: info.url,
      uptime: Math.floor((Date.now() - info.timestamp) / 1000),
    })
  } else {
    output({ running: false })
  }
}
