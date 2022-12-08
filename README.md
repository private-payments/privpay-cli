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

# Alice notifying Bob and deriving the first stealth address
$ privpay sender notify -r 0 -t p2wpkh pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8
# <enter fe for seed hex>
OP_RETURN OP_PUSHBYTES_40 505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401
0: bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w

# Bob deriving same stealth address
privpay receiver decode -P 6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401
# <enter ff for seed hex>
0: bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w 03e669bd1705691a080840b07d76713d040934a37f2e8dde2fe02f5d3286a49219 L1fJmsaHyM96XrpHs765ueXfmv1V7TiNgWJHS8ZsTgfVFvLd1TcU
```

## JSON Output

Running the previous example with the `--json` output flag produces more detailed output.

### Generate Bob's Payment Code

```bash
$ privpay receiver code --json -t p2pkh -t p2wpkh
# <enter ff for seed hex>
```

```json
{
  "payment_code": "pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8",
  "account": 0,
  "bip32_path": "m/351'/0'/0'",
  "address_types": [
    "p2wpkh",
    "p2pkh"
  ]
}
```

### Alice notifying Bob and deriving the first stealth address

```bash
$ privpay sender notify --json -r 0 -t p2wpkh pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8
# <enter fe for seed hex>
```

```json
{
  "receiver": {
    "payment_code": "pay1qqpsxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys6xv2gs8",
    "index": 0
  },
  "sender": {
    "account": 0,
    "bip32_path": "m/351'/0'/0'"
  },
  "notification": {
    "scriptpubkey": "6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401",
    "payload": "505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401",
    "asm": "OP_RETURN OP_PUSHBYTES_40 505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401"
  },
  "addresses": [
    {
      "address": "bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w",
      "index": 0
    }
  ]
}
```

### Bob deriving same stealth address

```bash
privpay receiver decode --json -P 6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401
# <enter ff for seed hex>
```

```json
{
  "receiver": {
    "payment_code": "pay1qqpqxq4730l4yre4lt3588eyt3f2lwggtfalvtgfns04a8smzkn7yys66cs29t",
    "account": 0,
    "bip32_path": "m/351'/0'/0'"
  },
  "notification": {
    "scriptpubkey": "6a28505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401",
    "payload": "505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401",
    "asm": "OP_RETURN OP_PUSHBYTES_40 505049cb55bb02e3217349724307eed5514b53b1f53f0802672a9913d9bbb76afecc86be23f46401"
  },
  "addresses": [
    {
      "address": "bc1qw7ld5h9tj2ruwxqvetznjfq9g5jyp0gjhrs30w",
      "index": 0,
      "public_key": "03e669bd1705691a080840b07d76713d040934a37f2e8dde2fe02f5d3286a49219",
      "private_key": "L1fJmsaHyM96XrpHs765ueXfmv1V7TiNgWJHS8ZsTgfVFvLd1TcU"
    }
  ]
}
```
