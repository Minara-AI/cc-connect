// Mirrors crates/cc-connect-core/src/message.rs::Message — same shape as
// chat-ui/src/types.ts. Hand-kept until the extension lifts the chat-ui
// shared module into a vendored package per design §5.

export const KIND_CHAT = 'chat';
export const KIND_FILE_DROP = 'file_drop';

export interface Message {
  id: string;
  author: string;
  nick?: string | null;
  ts: number;
  kind: string;
  body: string;
  blob_hash?: string | null;
  blob_size?: number | null;
}
