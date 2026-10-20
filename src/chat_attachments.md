# Chat attachments

Images, documents and other files can be sent in the [Peer-to-peer Chat](./chat.md) and in the [Dispute Chat](./dispute_chat.md). A file never travels over the relays. The sender encrypts it on the device, uploads the ciphertext to a [Blossom](https://github.com/hzrd149/blossom) server and sends a chat message that names it. That message is an ordinary inner kind 1, so the conversation's encryption, authentication and validation apply to it unchanged.

## File key

Files are encrypted with a key derived from the conversation's `K_conv`, using HKDF-SHA256 (RFC 5869) with an empty salt:

```
K_file = HKDF-SHA256(ikm = K_conv, info = "mostro:chat:file:v1", L = 32)
```

- The `ikm` is the 32-byte secret of `K_conv`. In the peer chat, that is the `K_conv` of [Key derivation](./chat.md#key-derivation). In the dispute chat, it is the `K_conv` each party derives with the admin ([Establishing the shared key](./dispute_chat.md#establishing-the-shared-key)).
- Both parties derive the same `K_file`, and so does a solver given `K_conv`.
- `K_file` is a symmetric key, not a secp256k1 secret key: any 32-byte output is valid, and there is no re-derivation rule.
- Implementations MUST NOT use the ECDH shared secret, `K_conv` or `K_sign` directly as the file key.

Deriving the file key from `K_conv` keeps the [dispute disclosure](./chat.md#dispute-disclosure) whole. The key that lets a solver read the conversation also opens its files, and `K_sign` is still never disclosed, so the grant stays read-only. The `info` string separates `K_file` from `K_conv`'s own use as a NIP-44 key.

Clients released before this page use a different key; see [Migration](#migration).

## Encryption

The file is encrypted with ChaCha20-Poly1305 (RFC 8439), with:

- the key `K_file`;
- a 12-byte nonce from a cryptographically secure random generator, new for every file. A nonce MUST NOT repeat under one key. Chosen at random, the chance of a repeat is negligible for the number of files a conversation carries;
- no associated data.

The blob is the nonce followed by the AEAD output:

```
blob = nonce (12 bytes) ‖ ciphertext ‖ tag (16 bytes)
```

A file is at most 25 MiB (26 214 400 bytes), so a blob is at most 26 214 428 bytes. Senders MUST refuse a larger file, and readers MUST NOT download a larger blob.

## Upload

The blob is uploaded with [BUD-02](https://github.com/hzrd149/blossom/blob/master/buds/02.md):

```
PUT <server>/upload
Content-Type: application/octet-stream
Authorization: Nostr <the authorization event, Base64url without padding>
```

The content type is always `application/octet-stream`: the blob is ciphertext, whatever the file was. The authorization is a Blossom kind 24242 event:

```json
{
  "id": "<Event Id>",
  "pubkey": "<a key generated for this upload>",
  "kind": 24242,
  "created_at": 1703021488,
  "content": "Upload attachment",
  "tags": [
    ["t", "upload"],
    ["x", "<SHA-256 of the blob, hex>"],
    ["expiration", "1703025088"]
  ],
  "sig": "<Signature of the upload key>"
}
```

The event MUST be signed by a key generated for that upload alone, never by the identity key or a trade key. Otherwise a server would see one pubkey behind every file of a trade, or of a user. `expiration` is a short window after `created_at`; existing clients use one hour.

[BUD-11](https://github.com/hzrd149/blossom/blob/master/buds/11.md) requires the event to be encoded as Base64url without padding, and senders SHOULD encode it that way. Existing clients send standard Base64 with padding, which the servers they use accept.

The server then serves the blob at `<server>/<sha256>` (BUD-01), where `<sha256>` is the lowercase hex SHA-256 of the blob. That URL is what the message carries. If the server answers with a blob descriptor whose `sha256` is not the blob's, the sender MUST treat the upload as failed.

Clients choose the servers. A suitable server accepts opaque blobs (a server that only takes media inspects the content and refuses ciphertext) and keeps them for as long as a dispute may need them.

## Message

Once the upload succeeds, the sender sends an inner kind 1 in the conversation whose `content` is one JSON object, in one of two shapes.

An image that readers can preview inline, JPEG or PNG:

```json
{
  "type": "image_encrypted",
  "blossom_url": "https://blossom.example/b1674191a88ec5cdd733e4240a81803105dc412d6c6708d53ab94fc248f4f553",
  "nonce": "0102030405060708090a0b0c",
  "mime_type": "image/jpeg",
  "original_size": 524288,
  "width": 1920,
  "height": 1080,
  "filename": "receipt.jpg",
  "encrypted_size": 524316
}
```

Any other file:

```json
{
  "type": "file_encrypted",
  "file_type": "document",
  "blossom_url": "https://blossom.example/b1674191a88ec5cdd733e4240a81803105dc412d6c6708d53ab94fc248f4f553",
  "nonce": "0102030405060708090a0b0c",
  "mime_type": "application/pdf",
  "original_size": 1048576,
  "filename": "bank.pdf",
  "encrypted_size": 1048604
}
```

| Field | Type | Meaning |
|-------|------|---------|
| `type` | string | `image_encrypted` or `file_encrypted`. |
| `file_type` | string | `file_encrypted` only: `image`, `video` or `document`. |
| `blossom_url` | string | The `https` URL of the blob. Its last path segment is the blob's SHA-256 in hex. |
| `nonce` | string | Hex of the blob's first 12 bytes. Readers decrypt with the nonce at the head of the blob, never with this field, which is sent only because existing readers require it. |
| `mime_type` | string | The type the sender declares. A label, not a fact: see [Receiving](#receiving). |
| `original_size` | integer | Size of the file, in bytes. |
| `encrypted_size` | integer | Size of the blob, in bytes: `original_size + 28`. |
| `filename` | string | The name the sender suggests for saving the file. |
| `width`, `height` | integer | `image_encrypted` only: the image's size, in pixels. |

Every field of a shape is REQUIRED, with the JSON type above: existing readers parse them strictly and treat a message that lacks one as text. Readers MUST ignore fields they do not know.

Which files a client offers to send is its own choice. Existing clients send JPEG, PNG and PDF, and v1 also sends MP4, MOV, AVI, DOC and DOCX. Senders SHOULD strip the metadata of an image before encrypting it: re-encoding the pixels drops EXIF, which can carry the position, the device and the time a photo was taken.

## Receiving

A reader treats an incoming kind 1 as an attachment only when its `content` starts with `{` and parses as one of the shapes above. Anything else is text. For an attachment:

1. `blossom_url` MUST be an `https` URL whose last path segment, before any extension, is 64 hex digits. Otherwise the blob is not fetched.
2. The URL's host MUST NOT be `localhost` or an IP address in a loopback, private, link-local or unspecified range. Readers SHOULD also refuse a host that resolves to such an address, where the platform exposes the resolution. The sender chose the URL: without this rule, a counterparty could make the reader send requests into the reader's own network. The hash check comes too late to prevent that, since it runs after the request.
3. Every redirect the reader follows MUST pass rules 1 and 2 again. Readers MAY refuse redirects altogether.
4. The reader MUST NOT download more than the blob limit, whatever `encrypted_size` says, and MAY refuse a message whose `encrypted_size` is already above it.
5. The SHA-256 of the downloaded blob MUST equal the hash in the URL before anything is decrypted. A server cannot substitute a blob.
6. The reader decrypts with `K_file`. If the AEAD rejects it, the reader tries the legacy key ([Migration](#migration)). If both fail, the file cannot be opened.
7. What the file is MUST be decided from the decrypted bytes, by their signature, never from `mime_type`, `file_type` or `filename`, which the sender chose. `filename` MUST be sanitized before it is used as a path.

The sender also chose the server, so fetching a blob can tell a server the counterparty runs the reader's IP address. Readers SHOULD fetch automatically only from servers they trust, such as the ones they upload to. They SHOULD fetch from any other host only when the user asks, and show that host. Requiring every host to be on the reader's own list would break delivery between clients that use different servers.

## Dispute disclosure

A solver given the peer chat's `K_conv` ([Dispute disclosure](./chat.md#dispute-disclosure)) reads the attachment messages along with the rest of the conversation. The solver derives `K_file` from that same key, downloads each blob, checks its hash and decrypts it. The inner signature of the message attributes the file to the party who sent it, and the hash in the URL binds that message to the blob.

In the dispute chat, each party already shares a `K_conv` with the solver, so a file sent there opens for the solver directly.

A file encrypted with the legacy key cannot be opened from `K_conv`. A party who needs the solver to see such a file sends it again in the dispute chat.

## Privacy considerations

- The server learns the blob's size (the file's plus 28 bytes, since nothing is padded), its hash, and the IP address of whoever uploads or fetches it. It learns nothing of the content, the file name or the trade.
- The URL comes from the counterparty, who can name a server it runs and so learn the reader's IP address when the blob is fetched. See [Receiving](#receiving).
- A new authorization key for every upload keeps a server from linking two uploads by their signer.
- Blobs stay on public servers for as long as the servers keep them. Anyone who later obtains `K_conv` or a trade private key can decrypt them: there is no forward secrecy.

## Migration

Clients released before this page encrypt files with a **legacy key**: the ECDH shared secret itself, used directly as the ChaCha20-Poly1305 key. That is `shared` in [Key derivation](./chat.md#key-derivation) for the peer chat, and the secret computed with the admin for the [dispute chat](./dispute_chat.md#establishing-the-shared-key). Everything else on this page is what those clients already do, so the message format does not change.

1. **Readers first.** Implementations MUST decrypt with `K_file` and fall back to the legacy key when the AEAD rejects it. The tag makes the second attempt safe: a wrong key fails authentication and yields nothing.
2. **Senders after.** Senders keep the legacy key until readers that accept `K_file` have been released, then switch to `K_file`. Implementations MUST agree on the date of that switch. After it, no file is encrypted with the legacy key.
3. Readers SHOULD keep the fallback for as long as they keep conversations from before the switch, since those files stay on the servers.

## Test vector

Derived from the trade keys of the [chat test vector](./chat.md#test-vector):

```
ECDH shared secret:  def6633a53d07d1e829484c4d4bdbbeed2f4b14c21743e63871c174338e39475
K_conv secret:       786210709b3077748490720afcf6db73f8f8e85308341c2912a5067ba5e5583d
K_file:              b79a086a84c216ca5ed02bad7ebc2599496dd971f2955d80d2d26f103533ec69

nonce:               0102030405060708090a0b0c
plaintext (UTF-8):   Mostro attachment vector
blob:                0102030405060708090a0b0ce1592c4e81e8360e63c169c9cb5b3809ff0bb26f5d367e6c76b4849b8b266f10f07bf14c79bd4cf9
```

Check `pub(K_conv)` against the chat test vector first, then derive `K_file` and decrypt the blob. A real sender never chooses its nonce; this one is fixed so that the blob is reproducible. The blob must not open with the ECDH shared secret, nor with the `K_conv` secret used directly.

The legacy key, for the [Migration](#migration) fallback, from the same trade keys. This vector was produced by running v1's own code (MostroP2P/mobile v1.4.2):

```
key (ECDH shared secret): def6633a53d07d1e829484c4d4bdbbeed2f4b14c21743e63871c174338e39475
nonce:                    0102030405060708090a0b0c
plaintext (UTF-8):        Mostro v1 attachment vector
blob:                     0102030405060708090a0b0cd56fcf60b9ccf71bd15877dc98c919bf9be866f8629872d02b1d4ef15dff60106750e95b4098ea21a1fc98
```
