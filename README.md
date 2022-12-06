# Private Payments CLI Tool

A command line tool to work with the Private Payments stealth address system for Bitcoin.

- Generate recipient payment codes
- Create sender notification payloads
- Decode notifications to reveal stealth addresses

## Installation

From Crates.io:

```bash
cargo install privpay
privpay -h
````

For local development

```bash
git clone https://github.com/private-payments/privpay-cli.git
cd privpay-cli
cargo build
```

## Example

Recreating the example from [BIP351](https://github.com/bitcoin/bips/blob/master/bip-0351.mediawiki):

```bash
# Generate Bob's Payment Code
$ privpay receiver code -t p2pkh -t p2wpkh
# <enter ff for seed hex>
pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8

# Alice notifying Bob
$ privpay sender notify -r 0 -t p2wpkh pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8
# <enter fe for seed hex>
6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401

# Alice deriving first stealth address
$ privpay sender address -r 0 -t p2wpkh pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8
# <enter fe for seed hex>
0: bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w

# Bob deriving same stealth address
privpay receiver decode -P 6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401`
# <enter ff for seed hex>
0: bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w 03e669bd1705691a080840b07d76713d040934a37f2e8dde2fe02f5d3286a49219 L1fJmsaHyM96XrpHs765ueXfmv1V7TiNgWJHS8ZsTgfVFvLd1TcU
```
