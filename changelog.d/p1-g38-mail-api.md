### Added

- New crate `khalid-mail`, the transport the newsletter, sign-in links and gated content call to send mail. This part sends through Resend's or Postmark's API with one POST per email through the shared outbound client, so its timeouts, size limits and private-network guard apply. The email's text and the key never reach a log or an error.
