**`SECURITY.md`** — обязательный файл для грантовых комиссий и аудиторов:

```markdown
# Security Policy

## Scope

This policy covers the Anchor smart contract in `programs/` and on-chain Token-2022 initialization scripts in `scripts/`.

## Security Guarantees

Upon Mainnet deployment and authority revocation:
- No admin key can mint new `$TAPR` tokens.
- No admin key can freeze user token accounts.
- No admin key can modify the `0.05%` transfer fee rate.
- No admin key can upgrade the harvesting program bytecode.

## Reporting a Vulnerability

Please do **not** open public GitHub Issues for security vulnerabilities.
Report findings directly to `security@taperlab.net` with reproduction steps or a proof-of-concept test.
