import type { PenNode, PathNode } from '@/types/pen'
import { useDocumentStore } from '@/stores/document-store'
import featherData from '@iconify-json/feather/icons.json'
import {
  clamp,
  toSizeNumber,
  toStrokeThicknessNumber,
  extractPrimaryColor,
} from './generation-utils'

// ---------------------------------------------------------------------------
// Core UI icon paths (Lucide-style, 24×24 viewBox)
// Hand-picked high-frequency icons for guaranteed instant sync resolution.
// Feather icons are added at module init from the bundled @iconify-json/feather.
// ---------------------------------------------------------------------------
const ICON_PATH_MAP: Record<string, { d: string; style: 'stroke' | 'fill'; iconId?: string }> = {
  // Navigation & actions
  menu:           { d: 'M4 6h16M4 12h16M4 18h16', style: 'stroke' },
  x:              { d: 'M18 6L6 18M6 6l12 12', style: 'stroke' },
  close:          { d: 'M18 6L6 18M6 6l12 12', style: 'stroke', iconId: 'lucide:x' },
  check:          { d: 'M20 6L9 17l-5-5', style: 'stroke' },
  plus:           { d: 'M12 5v14M5 12h14', style: 'stroke' },
  add:            { d: 'M12 5v14M5 12h14', style: 'stroke', iconId: 'lucide:plus' },
  minus:          { d: 'M5 12h14', style: 'stroke' },
  search:         { d: 'M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.35-4.35', style: 'stroke' },
  arrowright:     { d: 'M5 12h14M12 5l7 7-7 7', style: 'stroke', iconId: 'lucide:arrow-right' },
  arrowleft:      { d: 'M19 12H5M12 19l-7-7 7-7', style: 'stroke', iconId: 'lucide:arrow-left' },
  arrowup:        { d: 'M12 19V5M5 12l7-7 7 7', style: 'stroke', iconId: 'lucide:arrow-up' },
  arrowdown:      { d: 'M12 5v14M19 12l-7 7-7-7', style: 'stroke', iconId: 'lucide:arrow-down' },
  chevronright:   { d: 'M9 18l6-6-6-6', style: 'stroke', iconId: 'lucide:chevron-right' },
  chevronleft:    { d: 'M15 18l-6-6 6-6', style: 'stroke', iconId: 'lucide:chevron-left' },
  chevrondown:    { d: 'M6 9l6 6 6-6', style: 'stroke', iconId: 'lucide:chevron-down' },
  chevronup:      { d: 'M18 15l-6-6-6 6', style: 'stroke', iconId: 'lucide:chevron-up' },
  // People & account
  star:           { d: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z', style: 'fill' },
  heart:          { d: 'M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 000-7.78z', style: 'stroke' },
  like:           { d: 'M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14zM7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3', style: 'stroke', iconId: 'lucide:thumbs-up' },
  thumbsup:       { d: 'M14 9V5a3 3 0 00-3-3l-4 9v11h11.28a2 2 0 002-1.7l1.38-9a2 2 0 00-2-2.3H14zM7 22H4a2 2 0 01-2-2v-7a2 2 0 012-2h3', style: 'stroke', iconId: 'lucide:thumbs-up' },
  home:           { d: 'M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9zM9 22V12h6v10', style: 'stroke' },
  user:           { d: 'M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', style: 'stroke' },
  profile:        { d: 'M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', style: 'stroke', iconId: 'lucide:user' },
  users:          { d: 'M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2M9 11a4 4 0 100-8 4 4 0 000 8zM23 21v-2a4 4 0 00-3-3.87M16 3.13a4 4 0 010 7.75', style: 'stroke', iconId: 'lucide:users' },
  avatar:         { d: 'M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', style: 'stroke', iconId: 'lucide:user' },
  // System & settings
  settings:       { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  gear:           { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke', iconId: 'lucide:settings' },
  mail:           { d: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zm16 2l-10 7L2 6', style: 'stroke' },
  email:          { d: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zm16 2l-10 7L2 6', style: 'stroke', iconId: 'lucide:mail' },
  eye:            { d: 'M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  lock:           { d: 'M19 11H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM7 11V7a5 5 0 0110 0v4', style: 'stroke' },
  bell:           { d: 'M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0', style: 'stroke' },
  notification:   { d: 'M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0', style: 'stroke', iconId: 'lucide:bell' },
  shield:         { d: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z', style: 'stroke', iconId: 'lucide:shield' },
  zap:            { d: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z', style: 'fill', iconId: 'lucide:zap' },
  bolt:           { d: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z', style: 'fill', iconId: 'lucide:zap' },
  // Media & content
  play:           { d: 'M5 3l14 9-14 9V3z', style: 'fill' },
  pause:          { d: 'M6 4h4v16H6zM14 4h4v16h-4z', style: 'fill' },
  download:       { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3', style: 'stroke' },
  upload:         { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12', style: 'stroke' },
  image:          { d: 'M21 3H3a2 2 0 00-2 2v14a2 2 0 002 2h18a2 2 0 002-2V5a2 2 0 00-2-2zM8.5 10a1.5 1.5 0 100-3 1.5 1.5 0 000 3zM21 15l-5-5L5 21', style: 'stroke', iconId: 'lucide:image' },
  photo:          { d: 'M21 3H3a2 2 0 00-2 2v14a2 2 0 002 2h18a2 2 0 002-2V5a2 2 0 00-2-2zM8.5 10a1.5 1.5 0 100-3 1.5 1.5 0 000 3zM21 15l-5-5L5 21', style: 'stroke', iconId: 'lucide:image' },
  camera:         { d: 'M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2v11zM12 17a4 4 0 100-8 4 4 0 000 8z', style: 'stroke', iconId: 'lucide:camera' },
  video:          { d: 'M23 7l-7 5 7 5V7zM1 5h15a2 2 0 012 2v10a2 2 0 01-2 2H1a2 2 0 01-2-2V7a2 2 0 012-2z', style: 'stroke', iconId: 'lucide:video' },
  // Communication
  message:        { d: 'M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z', style: 'stroke', iconId: 'lucide:message-square' },
  chat:           { d: 'M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2v10z', style: 'stroke', iconId: 'lucide:message-square' },
  phone:          { d: 'M22 16.92v3a2 2 0 01-2.18 2 19.79 19.79 0 01-8.63-3.07A19.5 19.5 0 013.07 9.81a19.79 19.79 0 01-3.07-8.63A2 2 0 012 1h3a2 2 0 012 1.72c.127.96.361 1.903.7 2.81a2 2 0 01-.45 2.11L6.09 8.91a16 16 0 006 6l1.27-1.27a2 2 0 012.11-.45c.907.339 1.85.573 2.81.7A2 2 0 0122 16.92z', style: 'stroke', iconId: 'lucide:phone' },
  send:           { d: 'M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z', style: 'stroke' },
  share:          { d: 'M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8M16 6l-4-4-4 4M12 2v13', style: 'stroke', iconId: 'lucide:share' },
  globe:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z', style: 'stroke' },
  // Content & data
  code:           { d: 'M16 18l6-6-6-6M8 6l-6 6 6 6', style: 'stroke' },
  bookmark:       { d: 'M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2v16z', style: 'stroke', iconId: 'lucide:bookmark' },
  tag:            { d: 'M20.59 13.41l-7.17 7.17a2 2 0 01-2.83 0L2 12V2h10l8.59 8.59a2 2 0 010 2.82zM7 7h.01', style: 'stroke', iconId: 'lucide:tag' },
  link:           { d: 'M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71', style: 'stroke', iconId: 'lucide:link' },
  externallink:   { d: 'M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3', style: 'stroke', iconId: 'lucide:external-link' },
  copy:           { d: 'M20 9h-9a2 2 0 00-2 2v9a2 2 0 002 2h9a2 2 0 002-2V11a2 2 0 00-2-2zM5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1', style: 'stroke', iconId: 'lucide:copy' },
  clipboard:      { d: 'M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2M9 2h6a1 1 0 011 1v2a1 1 0 01-1 1H9a1 1 0 01-1-1V3a1 1 0 011-1z', style: 'stroke', iconId: 'lucide:clipboard' },
  edit:           { d: 'M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z', style: 'stroke', iconId: 'lucide:edit' },
  pencil:         { d: 'M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z', style: 'stroke', iconId: 'lucide:pencil' },
  trash:          { d: 'M3 6h18M8 6V4h8v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6M10 11v6M14 11v6', style: 'stroke', iconId: 'lucide:trash-2' },
  delete:         { d: 'M3 6h18M8 6V4h8v2M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6M10 11v6M14 11v6', style: 'stroke', iconId: 'lucide:trash-2' },
  // Time & location
  calendar:       { d: 'M8 2v4M16 2v4M3 10h18M5 4h14a2 2 0 012 2v14a2 2 0 01-2 2H5a2 2 0 01-2-2V6a2 2 0 012-2z', style: 'stroke', iconId: 'lucide:calendar' },
  clock:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 6v6l4 2', style: 'stroke', iconId: 'lucide:clock' },
  timer:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 6v6l4 2', style: 'stroke', iconId: 'lucide:clock' },
  mappin:         { d: 'M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0zM12 13a3 3 0 100-6 3 3 0 000 6z', style: 'stroke', iconId: 'lucide:map-pin' },
  location:       { d: 'M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0zM12 13a3 3 0 100-6 3 3 0 000 6z', style: 'stroke', iconId: 'lucide:map-pin' },
  map:            { d: 'M1 6v16l7-4 8 4 7-4V2l-7 4-8-4-7 4zM8 2v16M16 6v16', style: 'stroke', iconId: 'lucide:map' },
  // Analytics & status
  barchart:       { d: 'M18 20V10M12 20V4M6 20v-4', style: 'stroke', iconId: 'lucide:bar-chart-2' },
  chart:          { d: 'M18 20V10M12 20V4M6 20v-4', style: 'stroke', iconId: 'lucide:bar-chart-2' },
  analytics:      { d: 'M18 20V10M12 20V4M6 20v-4', style: 'stroke', iconId: 'lucide:bar-chart-2' },
  trendingup:     { d: 'M23 6l-9.5 9.5-5-5L1 18M17 6h6v6', style: 'stroke', iconId: 'lucide:trending-up' },
  activity:       { d: 'M22 12h-4l-3 9L9 3l-3 9H2', style: 'stroke', iconId: 'lucide:activity' },
  info:           { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 8v4M12 16h.01', style: 'stroke', iconId: 'lucide:info' },
  alert:          { d: 'M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01', style: 'stroke', iconId: 'lucide:alert-triangle' },
  warning:        { d: 'M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0zM12 9v4M12 17h.01', style: 'stroke', iconId: 'lucide:alert-triangle' },
  help:           { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01', style: 'stroke', iconId: 'lucide:help-circle' },
  question:       { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01', style: 'stroke', iconId: 'lucide:help-circle' },
  checkcircle:    { d: 'M22 11.08V12a10 10 0 11-5.93-9.14M22 4L12 14.01l-3-3', style: 'stroke', iconId: 'lucide:check-circle' },
  refresh:        { d: 'M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15', style: 'stroke', iconId: 'lucide:refresh-cw' },
  reload:         { d: 'M23 4v6h-6M1 20v-6h6M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15', style: 'stroke', iconId: 'lucide:refresh-cw' },
  filter:         { d: 'M22 3H2l8 9.46V19l4 2V12.46L22 3z', style: 'stroke', iconId: 'lucide:filter' },
  // Layout & UI
  grid:           { d: 'M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z', style: 'stroke', iconId: 'lucide:grid' },
  list:           { d: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01', style: 'stroke', iconId: 'lucide:list' },
  layers:         { d: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5', style: 'stroke', iconId: 'lucide:layers' },
  // Commerce
  creditcard:     { d: 'M21 4H3a2 2 0 00-2 2v12a2 2 0 002 2h18a2 2 0 002-2V6a2 2 0 00-2-2zM1 10h22', style: 'stroke', iconId: 'lucide:credit-card' },
  cart:           { d: 'M9 22a1 1 0 100-2 1 1 0 000 2zM20 22a1 1 0 100-2 1 1 0 000 2zM1 1h4l2.68 13.39a2 2 0 002 1.61h9.72a2 2 0 002-1.61L23 6H6', style: 'stroke', iconId: 'lucide:shopping-cart' },
  shoppingcart:   { d: 'M9 22a1 1 0 100-2 1 1 0 000 2zM20 22a1 1 0 100-2 1 1 0 000 2zM1 1h4l2.68 13.39a2 2 0 002 1.61h9.72a2 2 0 002-1.61L23 6H6', style: 'stroke', iconId: 'lucide:shopping-cart' },
  award:          { d: 'M12 15a7 7 0 100-14 7 7 0 000 14zM8.21 13.89L7 23l5-3 5 3-1.21-9.12', style: 'stroke', iconId: 'lucide:award' },
  // Misc
  dot:            { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill', iconId: 'lucide:circle' },
  bullet:         { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill', iconId: 'lucide:circle' },
  point:          { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill', iconId: 'lucide:circle' },
  circlefill:     { d: 'M12 12m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0', style: 'fill', iconId: 'lucide:circle' },
}

// ---------------------------------------------------------------------------
// Feather icon set — bundled from @iconify-json/feather (286 icons, all stroke)
// Populated at module init so AI-generated designs never need async network fetches
// for standard Feather icons.
// ---------------------------------------------------------------------------

/**
 * Parse a Feather SVG body string into a compound SVG path `d` string.
 * Feather uses <path>, <circle>, <rect>, <ellipse> (all inside optional <g>).
 * All elements are converted to equivalent path commands and joined.
 */
function featherBodyToPathD(body: string): string | null {
  const parts: string[] = []

  // <path d="...">
  const pathRe = /\bd="([^"]+)"/g
  let m: RegExpExecArray | null
  while ((m = pathRe.exec(body)) !== null) parts.push(m[1])

  // <circle cx="x" cy="y" r="r"> → two half-arcs forming a closed circle
  const circleRe = /<circle[^>]+>/g
  while ((m = circleRe.exec(body)) !== null) {
    const tag = m[0]
    const cx = parseFloat(tag.match(/\bcx="([^"]+)"/)?.[1] ?? 'NaN')
    const cy = parseFloat(tag.match(/\bcy="([^"]+)"/)?.[1] ?? 'NaN')
    const r  = parseFloat(tag.match(/\br="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(cx) && !isNaN(cy) && !isNaN(r)) {
      parts.push(`M ${cx - r} ${cy} a ${r} ${r} 0 1 0 ${r * 2} 0 a ${r} ${r} 0 1 0 ${-r * 2} 0 Z`)
    }
  }

  // <ellipse cx="x" cy="y" rx="rx" ry="ry">
  const ellipseRe = /<ellipse[^>]+>/g
  while ((m = ellipseRe.exec(body)) !== null) {
    const tag = m[0]
    const cx = parseFloat(tag.match(/\bcx="([^"]+)"/)?.[1] ?? 'NaN')
    const cy = parseFloat(tag.match(/\bcy="([^"]+)"/)?.[1] ?? 'NaN')
    const rx = parseFloat(tag.match(/\brx="([^"]+)"/)?.[1] ?? 'NaN')
    const ry = parseFloat(tag.match(/\bry="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(cx) && !isNaN(cy) && !isNaN(rx) && !isNaN(ry)) {
      parts.push(`M ${cx - rx} ${cy} a ${rx} ${ry} 0 1 0 ${rx * 2} 0 a ${rx} ${ry} 0 1 0 ${-rx * 2} 0 Z`)
    }
  }

  // <rect x="x" y="y" width="w" height="h" rx="r">
  const rectRe = /<rect[^>]+>/g
  while ((m = rectRe.exec(body)) !== null) {
    const tag = m[0]
    const x  = parseFloat(tag.match(/\bx="([^"]+)"/)?.[1] ?? '0') || 0
    const y  = parseFloat(tag.match(/\by="([^"]+)"/)?.[1] ?? '0') || 0
    const w  = parseFloat(tag.match(/\bwidth="([^"]+)"/)?.[1] ?? 'NaN')
    const h  = parseFloat(tag.match(/\bheight="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(w) && !isNaN(h)) {
      const rx = parseFloat(tag.match(/\brx="([^"]+)"/)?.[1] ?? '0') || 0
      if (rx > 0) {
        parts.push(
          `M ${x + rx} ${y} L ${x + w - rx} ${y} Q ${x + w} ${y} ${x + w} ${y + rx}` +
          ` L ${x + w} ${y + h - rx} Q ${x + w} ${y + h} ${x + w - rx} ${y + h}` +
          ` L ${x + rx} ${y + h} Q ${x} ${y + h} ${x} ${y + h - rx}` +
          ` L ${x} ${y + rx} Q ${x} ${y} ${x + rx} ${y} Z`,
        )
      } else {
        parts.push(`M ${x} ${y} L ${x + w} ${y} L ${x + w} ${y + h} L ${x} ${y + h} Z`)
      }
    }
  }

  // <line x1="x1" y1="y1" x2="x2" y2="y2"> → M x1 y1 L x2 y2
  const lineRe = /<line[^>]+>/g
  while ((m = lineRe.exec(body)) !== null) {
    const tag = m[0]
    const x1 = parseFloat(tag.match(/\bx1="([^"]+)"/)?.[1] ?? 'NaN')
    const y1 = parseFloat(tag.match(/\by1="([^"]+)"/)?.[1] ?? 'NaN')
    const x2 = parseFloat(tag.match(/\bx2="([^"]+)"/)?.[1] ?? 'NaN')
    const y2 = parseFloat(tag.match(/\by2="([^"]+)"/)?.[1] ?? 'NaN')
    if (!isNaN(x1) && !isNaN(y1) && !isNaN(x2) && !isNaN(y2)) {
      parts.push(`M ${x1} ${y1} L ${x2} ${y2}`)
    }
  }

  // <polyline points="x1,y1 x2,y2 ..."> → M x1 y1 L x2 y2 ...
  // <polygon points="..."> → same but closed with Z
  const polyRe = /<(polyline|polygon)([^>]+)>/g
  while ((m = polyRe.exec(body)) !== null) {
    const tag = m[0]
    const closed = m[1] === 'polygon'
    const pointsAttr = tag.match(/\bpoints="([^"]+)"/)?.[1]
    if (!pointsAttr) continue
    const coords = pointsAttr.trim().split(/[\s,]+/).map(Number)
    if (coords.length < 4 || coords.some(isNaN)) continue
    const cmds: string[] = [`M ${coords[0]} ${coords[1]}`]
    for (let i = 2; i + 1 < coords.length; i += 2) {
      cmds.push(`L ${coords[i]} ${coords[i + 1]}`)
    }
    if (closed) cmds.push('Z')
    parts.push(cmds.join(' '))
  }

  return parts.length > 0 ? parts.join(' ') : null
}

// Populate ICON_PATH_MAP with all 286 Feather icons at module load time.
// Keys are stored both in original kebab-case and normalized (no separator)
// form to match the icon resolver's name normalization.
;(function initFeatherIcons() {
  const icons = (featherData as { icons: Record<string, { body: string }> }).icons
  for (const [name, icon] of Object.entries(icons)) {
    const d = featherBodyToPathD(icon.body)
    if (!d) continue
    const iconId = `feather:${name}`
    const entry = { d, style: 'stroke' as const, iconId }
    // kebab-case key (e.g. "arrow-right") — for direct lookup in icon picker
    if (!ICON_PATH_MAP[name]) ICON_PATH_MAP[name] = entry
    // normalized key (e.g. "arrowright") — matches applyIconPathResolution normalization
    const normalized = name.replace(/-/g, '')
    if (!ICON_PATH_MAP[normalized]) ICON_PATH_MAP[normalized] = entry
  }
})()

// ---------------------------------------------------------------------------
// Pending async icon resolution tracking
// ---------------------------------------------------------------------------

/** Maps nodeId → normalized icon name for icons that need async resolution */
const pendingIconResolutions = new Map<string, string>()

/**
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 *
 * On local map miss for icon-like names, sets a generic placeholder and
 * records the node for async resolution via the Iconify API.
 */
export function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return
  const rawName = (node.name ?? node.id ?? '').toLowerCase()
    .replace(/[-_\s]+/g, '')       // normalize separators
    .replace(/(icon|logo)$/, '')   // strip trailing "icon" or "logo"

  let match = ICON_PATH_MAP[rawName]

  if (!match) {
    // 1. Try prefix fallback: "arrowdowncircle" → "arrowdown", "shieldcheck" → "shield"
    const prefixKey = findPrefixFallback(rawName)
    if (prefixKey) match = ICON_PATH_MAP[prefixKey]
  }

  if (!match) {
    // 2. Still no match — set placeholder and queue for async.
    // Use rawName if non-empty, else fall back to the full normalized name (handles
    // edge case where stripping "icon"/"logo" suffix leaves empty string, e.g. "Icon").
    const originalNormalized = (node.name ?? node.id ?? '').toLowerCase().replace(/[-_\s]+/g, '')
    const queueName = rawName || originalNormalized
    if (isIconLikeName(node.name ?? '', queueName)) {
      node.d = GENERIC_ICON_PATH
      if (!node.fill || node.fill.length === 0) {
        node.fill = [{ type: 'solid', color: extractPrimaryColor(node.stroke?.fill) ?? '#64748B' }]
      }
      // Record for async resolution
      pendingIconResolutions.set(node.id, queueName)
    }
    return
  }

  // Replace with verified path data and mark as resolved icon
  node.d = match.d
  node.iconId = match.iconId ?? `feather:${rawName}`
  applyIconStyle(node, match.style)
}

const EMOJI_REGEX = /[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F]/gu
const GENERIC_ICON_PATH = 'M12 3l2.6 5.27 5.82.84-4.2 4.09.99 5.8L12 16.9l-5.21 2.73.99-5.8-4.2-4.09 5.82-.84L12 3z'

export function applyNoEmojiIconHeuristic(node: PenNode): void {
  if (node.type !== 'text') return
  if (typeof node.content !== 'string' || !node.content) return

  EMOJI_REGEX.lastIndex = 0
  if (!EMOJI_REGEX.test(node.content)) return
  EMOJI_REGEX.lastIndex = 0
  const cleaned = node.content.replace(EMOJI_REGEX, '').replace(/\s{2,}/g, ' ').trim()
  if (cleaned.length > 0) {
    node.content = cleaned
    return
  }

  const iconSize = clamp(toSizeNumber(node.height, toSizeNumber(node.width, node.fontSize ?? 20)), 14, 24)
  const iconFill = extractPrimaryColor('fill' in node ? node.fill : undefined) ?? '#64748B'
  const replacement: PenNode = {
    id: node.id,
    type: 'path',
    name: `${node.name ?? 'Icon'} Path`,
    d: GENERIC_ICON_PATH,
    width: iconSize,
    height: iconSize,
    fill: [{ type: 'solid', color: iconFill }],
  } as PenNode

  if (typeof node.x === 'number') replacement.x = node.x
  if (typeof node.y === 'number') replacement.y = node.y
  if (typeof node.opacity === 'number') replacement.opacity = node.opacity
  if (typeof node.rotation === 'number') replacement.rotation = node.rotation
  replaceNode(node, replacement)
}

// ---------------------------------------------------------------------------
// Async icon resolution via Iconify API proxy
// ---------------------------------------------------------------------------

/**
 * Resolve pending icons asynchronously after streaming completes.
 * Walks the subtree rooted at `rootNodeId`, collects pending entries,
 * fetches from `/api/ai/icon` in parallel, and updates nodes in store.
 */
export async function resolveAsyncIcons(rootNodeId: string): Promise<void> {
  if (pendingIconResolutions.size === 0) return

  const { getNodeById, updateNode } = useDocumentStore.getState()

  // Collect pending entries that belong to this subtree
  const entries: Array<{ nodeId: string; iconName: string }> = []
  collectPendingInSubtree(rootNodeId, getNodeById, entries)
  if (entries.length === 0) return

  // Fetch all in parallel
  const results = await Promise.allSettled(
    entries.map(async ({ nodeId, iconName }) => {
      const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(iconName)}`)
      if (!res.ok) return { nodeId, icon: null }
      const data = (await res.json()) as {
        icon: { d: string; style: 'stroke' | 'fill'; width: number; height: number; iconId?: string } | null
      }
      return { nodeId, icon: data.icon }
    }),
  )

  // Apply resolved icons to the store
  for (const result of results) {
    if (result.status !== 'fulfilled') continue
    const { nodeId, icon } = result.value
    pendingIconResolutions.delete(nodeId)

    if (!icon) continue
    const node = getNodeById(nodeId)
    if (!node || node.type !== 'path') continue

    // Build update payload with resolved path + correct styling
    const update: Partial<PenNode> = { d: icon.d }
    if (icon.iconId) (update as Partial<PathNode>).iconId = icon.iconId
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'

    if (icon.style === 'stroke') {
      const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
      update.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
      update.fill = []
    } else {
      update.fill = [{ type: 'solid', color: existingColor }]
    }

    updateNode(nodeId, update)
  }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Check if a name looks like an icon reference (not just any path node). */
function isIconLikeName(originalName: string, normalized: string): boolean {
  // Explicit icon/logo suffix in original name
  if (/icon|logo/i.test(originalName)) return true
  // Short normalized name (likely an icon name, not a complex path description)
  if (normalized.length > 0 && normalized.length <= 30) return true
  return false
}

/** Apply stroke/fill styling to a resolved icon node (caller must ensure path type). */
function applyIconStyle(
  node: PathNode,
  style: 'stroke' | 'fill',
): void {
  if (style === 'stroke') {
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
    const strokeColor = extractPrimaryColor(node.stroke?.fill)
    // Ensure stroke is renderable for line icons
    if (!node.stroke || strokeWidth <= 0 || !strokeColor) {
      node.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
    }
    // Line icons should NOT have opaque fill (transparent to show stroke only)
    if (node.fill && node.fill.length > 0) {
      // Move fill color to stroke if stroke has no color
      const fillColor = extractPrimaryColor(node.fill)
      if (fillColor && node.stroke) {
        node.stroke.fill = [{ type: 'solid', color: fillColor }]
      }
      node.fill = []
    }
  } else {
    // Fill icons must always keep a visible fill.
    const fillColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    node.fill = [{ type: 'solid', color: fillColor }]
    // Remove non-renderable stroke definitions to avoid transparent-only paths.
    if (node.stroke && toStrokeThicknessNumber(node.stroke, 0) <= 0) {
      node.stroke = undefined
    }
  }
}

/** Walk subtree and collect entries from pendingIconResolutions. */
function collectPendingInSubtree(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
  out: Array<{ nodeId: string; iconName: string }>,
): void {
  const iconName = pendingIconResolutions.get(nodeId)
  if (iconName) {
    out.push({ nodeId, iconName })
  }

  const node = getNodeById(nodeId)
  if (!node || !('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    collectPendingInSubtree(child.id, getNodeById, out)
  }
}

function replaceNode(target: PenNode, replacement: PenNode): void {
  const targetRecord = target as unknown as Record<string, unknown>
  for (const key of Object.keys(target)) {
    delete targetRecord[key]
  }
  Object.assign(targetRecord, replacement as unknown as Record<string, unknown>)
}

// ---------------------------------------------------------------------------
// Available icon names export — used by AI prompts to constrain icon selection
// ---------------------------------------------------------------------------

/**
 * Sorted list of all available Feather icon names (kebab-case).
 * These are guaranteed to resolve instantly without any network request.
 */
export const AVAILABLE_FEATHER_ICONS: readonly string[] = Object.keys(
  (featherData as { icons: Record<string, unknown> }).icons,
).sort()

/**
 * Try to resolve an unknown normalized icon name by finding the longest
 * known icon key that the name starts with (prefix match, min 4 chars).
 * e.g. "arrowdowncircle" → "arrowdown", "shieldcheck" → "shield"
 */
function findPrefixFallback(normalizedName: string): string | null {
  let best: string | null = null
  let bestLen = 3 // require at least 4-char match
  for (const key of Object.keys(ICON_PATH_MAP)) {
    if (key.length > bestLen && normalizedName.startsWith(key)) {
      best = key
      bestLen = key.length
    }
  }
  return best
}
