use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::str::FromStr;

use bip351::*;
use bitcoin::hashes::hex::{FromHex, ToHex};
use bitcoin::{Network, Script};
use clap::{Args, Parser, Subcommand, ValueEnum};
use dialoguer::Password;
use secstr::SecUtf8;

/// Private Payments (BIP351) Helper Tool
///
/// The Private Payments stealth address protocol works in three phases:
///
/// 1. The receiver generates a payment code, derived from the seed hex of their HD wallet.
///
/// 2. The sender generates notification information and transmits it as an OP_RETURN output in the
/// Bitcoin blockchain or out-of-band to the receiver directly. The sender also derives stealth
/// addresses that only the receiver may spend.
///
/// 3. The receiver decodes the notificaton payload and derives the same stealth addresses that
/// the sender will use for payments. Only the receiver knows the private keys for these addresses.
#[derive(Debug, Parser)]
#[command(version)]
#[command(name = "privpay")]
#[command(bin_name = "privpay")]
enum Cli {
    /// Generate payment codes, decode notifications
    Receiver {
        #[command(subcommand)]
        command: Receiver,
    },
    /// Create notifications and stealth addresses
    Sender {
        #[command(subcommand)]
        command: Sender,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum Receiver {
    /// Generate a payment code
    Code {
        #[command(flatten)]
        common_args: CommonArgs,
        #[command(flatten)]
        address_types: AddressTypesArg,
    },
    /// Generate stealth addresses from a notification payload
    Decode {
        /// The notification payload
        notification: String,
        #[command(flatten)]
        common_args: CommonArgs,
        #[command(flatten)]
        address_types: AddressTypesArg,
        #[command(flatten)]
        address_range: AddressRangeArgs,
        /// Show the private key for each generated address
        #[arg(short = 'P', default_value_t = false)]
        show_private_key: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum Sender {
    /// Generate a notification payload and stealth addresses
    Notify {
        #[command(flatten)]
        common_args: CommonArgs,
        /// Recipient index should be uniqe per recipient
        #[arg(short = 'r')]
        recipient_index: u32,
        #[command(flatten)]
        address_type: AddressTypeArg,
        /// Payment code of the recipient
        recipient_payment_code: String,
        #[command(flatten)]
        address_range: AddressRangeArgs,
    },
}

#[derive(Debug, Clone, Args)]
struct CommonArgs {
    /// Which BIP32 account to use (m/351'/0'/x')
    #[arg(short, long, default_value_t = 0)]
    account: u32,
    /// Output results as JSON
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Debug, Clone, Args)]
struct AddressTypeArg {
    /// The address type committed to with this notification
    #[arg(short = 't', value_enum, rename_all = "lower", default_value_t = AddressType::default())]
    address_type: AddressType,
}

#[derive(Debug, Clone, Args)]
struct AddressTypesArg {
    /// Supported address types for receiving
    #[arg(short = 't', rename_all = "lower", default_values_t = vec![AddressType::default()])]
    address_types: Vec<AddressType>,
}

#[derive(Debug, Clone, Args)]
struct AddressRangeArgs {
    /// First stealth address index
    #[arg(short = 'i', default_value_t = 0)]
    start_address_index: u64,
    /// Last stealth address index
    #[arg(short = 'f')]
    last_address_index: Option<u64>,
}

impl Cli {
    fn run(self) -> Result<Output, Error> {
        match self {
            Self::Receiver { command } => command.run(),
            Self::Sender { command } => command.run(),
        }
    }
}

impl Receiver {
    fn run(self) -> Result<Output, Error> {
        let secp = bitcoin::secp256k1::Secp256k1::new();

        match self {
            Receiver::Code {
                common_args: CommonArgs { account, json },
                address_types: AddressTypesArg { address_types },
            } => {
                let seed = get_seed_hex()?;

                let accepted_addresses: HashSet<bip351::AddressType> =
                    address_types.into_iter().map(|t| t.into()).collect();

                let recipient = bip351::Recipient::from_seed(
                    &secp,
                    &seed,
                    Network::Bitcoin,
                    account,
                    accepted_addresses,
                )?;

                let payment_code = recipient.payment_code();

                let output = if json {
                    let address_types: Vec<String> = payment_code
                        .address_types()
                        .iter()
                        .map(|&a| AddressType::from(a).to_string())
                        .collect();
                    json::object! {
                        payment_code: payment_code.to_string(),
                        account: account,
                        bip32_path: format!("m/351'/0'/{}'", account),
                        address_types: address_types,
                    }
                    .into()
                } else {
                    payment_code.to_string().into()
                };

                Ok(output)
            }
            Receiver::Decode {
                notification,
                common_args: CommonArgs { account, json },
                address_types: AddressTypesArg { address_types },
                address_range:
                    AddressRangeArgs {
                        start_address_index,
                        last_address_index,
                    },
                show_private_key,
            } => {
                let bytes: Vec<u8> = FromHex::from_hex(&notification)?;
                let script = if bytes.starts_with(b"PP") {
                    Script::new_op_return(&bytes)
                } else {
                    Script::from(bytes)
                };

                let seed = get_seed_hex()?;

                let accepted_addresses: HashSet<bip351::AddressType> =
                    address_types.into_iter().map(|t| t.into()).collect();

                let recipient = bip351::Recipient::from_seed(
                    &secp,
                    &seed,
                    Network::Bitcoin,
                    account,
                    accepted_addresses,
                )?;

                if let Some(commitment) = recipient.detect_notification(&secp, &script) {
                    let range = index_range(start_address_index, last_address_index);
                    let output_capacity =
                        range.end().saturating_add(1).saturating_sub(*range.start()) as usize;

                    if json {
                        let mut addresses: Vec<json::JsonValue> =
                            Vec::with_capacity(output_capacity);
                        for c in range {
                            let (address, public_key, private_key) =
                                recipient.key_info(&secp, &commitment, c)?;
                            if show_private_key {
                                addresses.push(json::object! {
                                    address: address.to_string(),
                                    index: c,
                                    public_key: public_key.to_string(),
                                    private_key: private_key.to_string(),
                                });
                            } else {
                                addresses.push(json::object! {
                                    address: address.to_string(),
                                    index: c,
                                });
                            }
                        }

                        let output = json::object! {
                            receiver: json::object! {
                                payment_code: recipient.payment_code().to_string(),
                                account: account,
                                bip32_path: format!("m/351'/0'/{}'", account),
                            },
                            notification: json::object!{
                                scriptpubkey: script.to_hex(),
                                payload: script.to_bytes()[2..].to_hex(),
                                asm: script.asm(),
                            },
                            addresses: addresses,
                        };

                        return Ok(output.into());
                    } else {
                        let mut lines: Vec<String> = Vec::with_capacity(output_capacity);
                        for c in range {
                            let (address, public_key, private_key) =
                                recipient.key_info(&secp, &commitment, c)?;
                            if show_private_key {
                                lines.push(format!("{c}: {address} {public_key} {private_key}"));
                            } else {
                                lines.push(format!("{c}: {address}"));
                            }
                        }

                        return Ok(Output::Plain(lines.join("\n")));
                    }
                }

                Ok(Output::Empty)
            }
        }
    }
}

impl Sender {
    fn run(self) -> Result<Output, Error> {
        let secp = bitcoin::secp256k1::Secp256k1::new();

        match self {
            Sender::Notify {
                common_args: CommonArgs { account, json },
                recipient_index,
                address_type: AddressTypeArg { address_type },
                recipient_payment_code,
                address_range:
                    AddressRangeArgs {
                        start_address_index,
                        last_address_index,
                    },
            } => {
                let recipient = bip351::PaymentCode::from_str(&recipient_payment_code)?;

                let seed = get_seed_hex()?;

                let sender = bip351::Sender::from_seed(&secp, &seed, Network::Bitcoin, account)?;

                let (txout, commitment) =
                    sender.notify(&secp, &recipient, recipient_index, address_type.into())?;

                let range = index_range(start_address_index, last_address_index);
                let output_capacity =
                    range.end().saturating_add(1).saturating_sub(*range.start()) as usize;

                if json {
                    let mut addresses: Vec<json::JsonValue> = Vec::with_capacity(output_capacity);
                    for c in range {
                        let address = sender.address(&secp, &commitment, c)?;
                        addresses.push(json::object! {
                            address: address.to_string(),
                            index: c,
                        });
                    }

                    let output = json::object! {
                        receiver: json::object!{
                            payment_code: recipient_payment_code,
                            index: recipient_index,
                        },
                        sender: json::object!{
                            account: account,
                            bip32_path: format!("m/351'/0'/{}'", account),
                        },
                        notification: json::object!{
                            scriptpubkey: txout.script_pubkey.to_hex(),
                            payload: txout.script_pubkey.to_bytes()[2..].to_hex(),
                            asm: txout.script_pubkey.asm(),
                        },
                        addresses: addresses,
                    };

                    Ok(output.into())
                } else {
                    let mut lines: Vec<String> =
                        Vec::with_capacity(output_capacity.saturating_add(1));

                    lines.push(txout.script_pubkey.asm());
                    for c in range {
                        let address = sender.address(&secp, &commitment, c)?;
                        lines.push(format!("{c}: {address}"));
                    }

                    Ok(Output::Plain(lines.join("\n")))
                }
            }
        }
    }
}

fn get_seed_hex() -> Result<Vec<u8>, Error> {
    let seed_hex = SecUtf8::from(Password::new().with_prompt("Seed Hex").interact()?);
    let seed: Vec<u8> = FromHex::from_hex(seed_hex.unsecure())?;
    Ok(seed)
}

fn index_range(first_index: u64, last_index: Option<u64>) -> RangeInclusive<u64> {
    match last_index {
        Some(last_index) if last_index >= first_index => first_index..=last_index,
        _ => first_index..=first_index,
    }
}

#[derive(Debug, Clone, Default, ValueEnum)]
enum AddressType {
    P2pkh,
    #[default]
    P2wpkh,
    P2tr,
}

impl From<AddressType> for bip351::AddressType {
    fn from(t: AddressType) -> Self {
        match t {
            AddressType::P2pkh => Self::P2pkh,
            AddressType::P2wpkh => Self::P2wpkh,
            AddressType::P2tr => Self::P2tr,
        }
    }
}

impl From<bip351::AddressType> for AddressType {
    fn from(t: bip351::AddressType) -> Self {
        match t {
            bip351::AddressType::P2pkh => Self::P2pkh,
            bip351::AddressType::P2wpkh => Self::P2wpkh,
            bip351::AddressType::P2tr => Self::P2tr,
        }
    }
}

impl std::fmt::Display for AddressType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::P2pkh => "p2pkh",
            Self::P2wpkh => "p2wpkh",
            Self::P2tr => "p2tr",
        };

        s.fmt(f)
    }
}

enum Output {
    Empty,
    Json(json::JsonValue),
    Plain(String),
}

impl Output {
    fn print(self) {
        match self {
            Self::Empty => {}
            Self::Json(output) => println!("{}", json::stringify_pretty(output, 2)),
            Self::Plain(s) => println!("{s}"),
        }
    }
}

impl From<String> for Output {
    fn from(s: String) -> Self {
        Output::Plain(s)
    }
}

impl From<json::JsonValue> for Output {
    fn from(j: json::JsonValue) -> Self {
        Output::Json(j)
    }
}

fn main() -> Result<(), Error> {
    let command = Cli::parse();
    let output = command.run()?;

    output.print();

    Ok(())
}

#[derive(Debug)]
enum Error {
    Address(bitcoin::util::address::Error),
    Bip32(bitcoin::util::bip32::Error),
    Dialoguer(std::io::Error),
    Hex(bitcoin::hashes::hex::Error),
    PrivatePayment(bip351::Error),
}

impl From<bitcoin::util::address::Error> for Error {
    fn from(e: bitcoin::util::address::Error) -> Self {
        Self::Address(e)
    }
}

impl From<bitcoin::util::bip32::Error> for Error {
    fn from(e: bitcoin::util::bip32::Error) -> Self {
        Self::Bip32(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Dialoguer(e)
    }
}

impl From<bitcoin::hashes::hex::Error> for Error {
    fn from(e: bitcoin::hashes::hex::Error) -> Self {
        Self::Hex(e)
    }
}

impl From<bip351::Error> for Error {
    fn from(e: bip351::Error) -> Self {
        Self::PrivatePayment(e)
    }
}
