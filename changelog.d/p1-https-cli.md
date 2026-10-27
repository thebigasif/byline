### Added

- `khalid serve --domain blog.example.com` answers over HTTPS on port 443 with a free certificate from Let's Encrypt, which Khalid gets and renews by itself. Port 80 moves visitors to HTTPS. The first time, Khalid asks you to agree to Let's Encrypt's subscriber agreement; `--accept-terms` agrees without asking, for services and containers (KH-INS-4).
