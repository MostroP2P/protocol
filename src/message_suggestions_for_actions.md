# Message Suggestions for Actions

Below are suggestions for messages that clients can show to users when receiving specific actions. These messages can be customized, translated, enhanced with emojis, or modified to provide a better user experience. Clients should replace placeholders in `monospace` format with the corresponding values.

## Actions

- **new-order:**  
  Your offer has been published! Please wait until another user picks your order. It will be available for `expiration_hours` hours. You can cancel this order before another user picks it up by executing: `cancel`.

- **canceled:**  
  You have canceled the order ID: `id`.

- **pay-invoice:**  
  Please pay this hold invoice of `amount` Sats for `fiat_code` `fiat_amount` to start the operation. If you do not pay it within `expiration_seconds`, the trade will be canceled.

- **pay-bond-invoice:**  
  Please pay this **bond** hold invoice of `amount` Sats as a security deposit before the trade begins. The bond is separate from the trade escrow and is released when the trade completes normally. If you do not pay it within `expiration_seconds`, the take will be canceled — the order remains visible on the order book as `pending` and may be taken again by anyone.

- **add-bond-invoice:**  
  Please send me a Lightning invoice for `amount` Sats — this is your share of a slashed bond on order `id`. You have until `deadline` to submit it, or your share will be forfeited and the entire bond will be retained by the node.

- **bond-invoice-accepted:**  
  Your bond payout invoice has been received. The payment is being processed — you'll be notified once it completes.

- **bond-payout-completed:**  
  Your bond payout of `amount` Sats has been sent successfully. The funds should arrive in your Lightning wallet shortly.

- **bond-slashed:**  
  You have lost your anti-abuse bond of `amount` Sats for your order `id`.

- **add-invoice:**  
  Please send me an invoice for `amount` satoshis equivalent to `fiat_code` `fiat_amount`. This is where I will send the funds upon trade completion. If you don’t provide the invoice within `expiration_seconds`, the trade will be canceled.

- **waiting-seller-to-pay:**  
  Please wait. I’ve sent a payment request to the seller to send the Sats for the order ID: `id`. If the seller doesn’t complete the payment within `expiration_seconds`, the trade will be canceled.

- **waiting-buyer-invoice:**  
  Payment received! Your Sats are now "held" in your wallet. I’ve requested the buyer to provide an invoice. If they don’t do so within `expiration_seconds`, your Sats will return to your wallet, and the trade will be canceled.

- **buyer-invoice-accepted:**  
  The invoice has been successfully saved.

- **hold-invoice-payment-accepted:**  
  Contact the seller at `seller-npub` to arrange how to send `fiat_code` `fiat_amount` using `payment_method`. Once you send the fiat money, notify me with `fiat-sent`.

- **buyer-took-order:**  
  Contact the buyer at `buyer-npub` to inform them how to send `fiat_code` `fiat_amount` through `payment_method`. You’ll be notified when the buyer confirms the fiat payment. Afterward, you should verify if it has arrived. If the buyer does not respond, you can initiate a cancellation or a dispute. Remember, an administrator will NEVER contact you to resolve your order unless you open a dispute first.

- **fiat-sent-ok:**  
  - _To the buyer:_ I have informed `seller-npub` that you sent the fiat money. If the seller confirms receipt, they will release the funds. If they refuse, you can open a dispute. 
  - _To the seller:_ `buyer-npub` has informed you that they sent the fiat money. Once you confirm receipt, release the funds. After releasing, the money will go to the buyer and there will be no turning back, so only proceed if you are sure. If you want to release the Sats to the buyer, send me `release-order-message`.  

- **released:**  
  `seller-npub` has released the Sats! Expect your invoice to be paid shortly. Ensure your wallet is online to receive via Lightning Network.

- **purchase-completed:**  
  Your purchase of Bitcoin has been completed successfully. Your invoice has been paid. Enjoy sound money!

- **hold-invoice-payment-settled:**  
  Your sale of Bitcoin has been completed after confirming the payment from `buyer-npub`.

- **rate:**  
  Please rate your counterparty.

- **rate-received:**  
  The rating has been successfully saved.

- **reputation-exported:**
  Your reputation on this Mostro is ready to import. Review the figures before importing them.

- **reputation-imported:**
  Your reputation was imported. Counterparties now see it merged with your reputation on this Mostro.

- **cooperative-cancel-initiated-by-you:**  
  You’ve initiated the cancellation of order ID: `id`. Your counterparty must agree. If they do not respond, you can open a dispute. Note that no administrator will contact you regarding this cancellation unless you open a dispute first.

- **cooperative-cancel-initiated-by-peer:**  
  Your counterparty wants to cancel order ID: `id`. Send `cancel-order-message` to confirm. Note that no administrator will contact you regarding this cancellation unless you open a dispute first. If you agree on such cancellation, please send me `cancel-order-message`.

- **cooperative-cancel-accepted:**  
  Order ID: `id` has been successfully canceled.

- **dispute-initiated-by-you:**  
  You’ve initiated a dispute for order ID: `id`. A solver will be assigned soon. Once assigned, I will share their npub with you, and only they will be able to assist you. You may contact the solver directly.

- **dispute-initiated-by-peer:**  
  Your counterparty initiated a dispute for order ID: `id`. A solver will be assigned soon. Once assigned, I will share their npub with you, and only they will be able to assist you. You may contact the solver directly.

- **admin-took-dispute:**  
  - _Admin:_ Here are the details of the dispute: `details`. You need to determine which user is correct and decide whether to cancel or complete the order. Please note that your decision will be final and cannot be reversed.
  - _Users:_ Solver `admin-npub` will handle your dispute. You can contact them directly.

- **admin-canceled:**  
  - _Admin:_ You have canceled order ID: `id`.  
  - _Users:_ The admin has canceled order ID: `id`.

- **admin-settled:**  
  - _Admin:_ You have completed order ID: `id`.  
  - _Users:_ The admin has completed order ID: `id`.

- **payment-failed:**  
  I couldn’t send the Sats. I’ll retry `payment_attempts` times in `payment_retries_interval` minutes. Please ensure your node/wallet is online.

- **invoice-updated:**  
  The invoice has been successfully updated.

- **hold-invoice-payment-canceled:**  
  The invoice was canceled. Your Sats are available in your wallet again.

- **admin-add-solver:**  
  Solver `npub` has been successfully added.

- **cant-do:**  
  You are not allowed to perform the action: `action`.


## Cant Do Reasons

Mostro also handles messages with the `CantDo` action for various reasons. The details of the failure are included in the payload section of the event, providing a structured explanation of the issue. The reason travels as `snake_case`, e.g. `"payload": { "cant_do": "not_allowed_by_status" }`; match on that exact spelling. Below are suggested texts that clients can display to users based on the `CantDo` reason received:

- **invalid_trade_index:**  
  The provided trade index is invalid. Please ensure your client is synchronized and try again.

- **invalid_amount:**  
  The provided amount is invalid. Please verify it and try again.

- **invalid_invoice:**  
  The provided Lightning invoice is invalid. Please check the invoice details and try again.

- **invalid_peer:**  
  You are not authorized to perform this action.

- **invalid_order_status:**  
  The action cannot be completed due to the current order status. 

- **invalid_parameters:**  
  The action cannot be completed due to invalid parameters. Please review the provided values and try again.

- **invalid_pubkey:**  
  The action cannot be completed because the public key is invalid.

- **order_already_canceled:**  
  The action cannot be completed because the order has already been canceled.

- **cant_create_user:**  
  The action cannot be completed because the user could not be created.

- **is_not_your_dispute:**  
  This dispute is not assigned to you.

- **not_found:**  
  The requested dispute could not be found.

- **invalid_signature:**  
  The action cannot be completed because the signature is invalid.

- **is_not_your_order:**  
  This order does not belong to you.

- **not_allowed_by_status:**  
  The action cannot be completed because order Id `id` status is `order-status`.  

- **out_of_range_fiat_amount:**  
  The requested fiat amount is outside the acceptable range (`min_amount`–`max_amount`).

- **out_of_range_sats_amount:**  
  The allowed Sats amount for this Mostro is between min `min_order_amount` and max `max_order_amount`. Please enter an amount within this range.

- **too_many_requests:**
  User exceeds the allowed request rate.

- **invalid_fiat_currency:**
  Prevents proceeding with unrecognized fiat currencies.

- **maintenance_mode:**
  Mostro is in maintenance mode and is not accepting new orders or takes right now. Your existing orders are not affected and can still be completed or canceled. Please try again later.

- **invalid_action:**
  This Mostro does not support the requested action.

- **invalid_payload:**
  The message payload is missing or malformed. Please update your client and try again.

- **reputation_identity_required:**
  This needs your identity key, and the request did not prove it. Reputation is kept for an identity key only: if you use full privacy mode, switch to reputation mode to build one.

- **not_eligible_for_reputation_export:**
  Your account on this Mostro cannot export its reputation yet. You need at least 10 completed trades and 5 ratings received.

- **reputation_bound_to_other_identity:**
  Your reputation on this Mostro was already exported to another identity. Sign the request with that identity to move it.

- **invalid_reputation_rebind:**
  The authorization to move your reputation to a new identity is invalid or has expired. Please create a new one and try again.

- **invalid_reputation_attestation:**
  The reputation you are trying to import is invalid. Please request it again from its source.

- **untrusted_reputation_issuer:**
  This Mostro does not accept reputation from that source.

- **expired_reputation_attestation:**
  The reputation you are trying to import has expired. Please request it again from its source.

- **reputation_identity_mismatch:**
  This reputation was issued for a different identity than the one you are using.

- **reputation_already_imported:**
  This reputation was already imported on this Mostro.

- **unknown:**
  Mostro rejected the action for a reason this client does not recognize yet. Please update your client.

> **Forward compatibility.** A daemon never sends `unknown`; it is the value a client falls back to for a reason it does not know. Since `mostro-core` 0.14.6 `CantDoReason` carries a `#[serde(other)] Unknown` catch-all, so a client that deserializes with it reads an unrecognized reason as `unknown` instead of failing to parse the whole `cant_do` payload. Clients built against an older `mostro-core` fail to parse the payload and may drop the message without showing anything; clients with their own parser should map an unrecognized reason to `unknown` the same way. A daemon operator should therefore only enable features that emit new reasons (such as maintenance mode) once the clients it serves have caught up.
