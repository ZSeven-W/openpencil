import { randomBytes } from 'node:crypto';

// Prepare a host-mediated opening request. This never launches an OS browser or
// navigates from the sandbox; the current chat owns the built-in browser action.
export function prepareCanvasBrowser(controller, args = {}) {
  if (Object.keys(args).some(key => key !== 'theme') || !['light', 'dark'].includes(args.theme)) {
    throw new Error('Choose a light or dark theme; browser URLs cannot be supplied.');
  }
  controller.session.assertLive();
  // Reuse an in-flight receipt while the host processes the chat request.
  if (controller.browserLaunch?.state === 'pending' && Date.now() < controller.browserLaunch.expiresAt) {
    return { ...browserLaunchStatus(controller, { launchId: controller.browserLaunch.launchId }), panelUrl: controller.browserLaunch.panelUrl };
  }
  const launchId = randomBytes(16).toString('base64url');
  const url = new URL(controller.panel.url);
  url.hash = `theme=${args.theme}&launch=${launchId}`;
  controller.browserLaunch = { launchId, panelUrl: url.href, state: 'pending', expiresAt: Date.now() + 120_000 };
  return { ...browserLaunchStatus(controller, { launchId }), panelUrl: url.href };
}

export function browserLaunchStatus(controller, args = {}, connected = false) {
  const launch = controller.browserLaunch;
  if (Object.keys(args).some(key => key !== 'launchId') || typeof args.launchId !== 'string' ||
      !launch || launch.launchId !== args.launchId) throw new Error('Unknown browser launch.');
  if (launch.state === 'pending' && Date.now() >= launch.expiresAt) launch.state = 'unconfirmed';
  if (connected && launch.state === 'pending') launch.state = 'connected';
  return { launchId: launch.launchId, state: launch.state };
}
