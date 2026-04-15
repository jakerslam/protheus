// Production SDK transport surface: resident_ipc_authoritative only.
export {
  RESIDENT_IPC_TOPOLOGY,
  createResidentIpcTransport,
} from './transports/resident_ipc';
export const PRODUCTION_TRANSPORT_SURFACE = 'resident_ipc_only';
export type {
  ResidentIpcInvoker,
  ResidentIpcTransportOptions,
} from './transports/resident_ipc';
