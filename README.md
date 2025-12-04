# [xlauth][github-project]

[![Build][build-status-badge]][build-status-url]
[![GitHub Contributors][contributors-badge]][contributors-url]
[![GitHub Stars][stars-badge]][stars-url]
[![GitHub Forks][forks-badge]][forks-url]

A small utility to send TOTP authenticator codes to [XIV Launcher][xiv-launcher].

## Usage

#### Windows
```pwsh
xlauth-cli save AD2J0N3FDDJPWEH0ADJP6WK5EGG4PTBT
xlauth-cli launch
```

**Note:** `xlauth-cli.exe` is a thin wrapper around `xlauth.exe` to provide a reasonable command line experience on
Windows. Use `xlauth.exe` when creating a shortcut to prevent the command line window from appearing.

#### Linux
```bash
xlauth save AD2J0N3FDDJPWEH0ADJP6WK5EGG4PTBT
xlauth launch
```

## License

This project is licensed under the [MIT license][license].

<!-- References -->

[github-project]: https://github.com/alphaONE2/xlauth

[xiv-launcher]: https://goatcorp.github.io/

[license]: ./LICENSE

[build-status-url]: https://github.com/alphaONE2/xlauth/actions/workflows/rust.yml
[contributors-url]: https://github.com/alphaONE2/xlauth/graphs/contributors
[stars-url]: https://github.com/alphaONE2/xlauth/stargazers
[forks-url]: https://github.com/alphaONE2/xlauth/network/members

[build-status-badge]: https://github.com/alphaONE2/xlauth/actions/workflows/rust.yml/badge.svg?branch=main
[contributors-badge]: https://img.shields.io/github/contributors/alphaONE2/xlauth
[stars-badge]: https://img.shields.io/github/stars/alphaONE2/xlauth
[forks-badge]: https://img.shields.io/github/forks/alphaONE2/xlauth
