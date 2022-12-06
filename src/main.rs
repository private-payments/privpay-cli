use std::collections::HashSet;
use std::ops::RangeInclusive;
use std::str::FromStr;

use bip351::*;
use bitcoin::hashes::hex::{FromHex, ToHex};
use bitcoin::{Network, Script};
use clap::{Args, Parser, Subcommand, ValueEnum};
use dialoguer::Password;
use secstr::SecUtf8;

#[derive(Debug, Parser)]
#[command(name = "privpay")]
#[command(bin_name = "privpay")]
enum Cli {
    Receiver {
        #[command(subcommand)]
        command: Receiver,
    },
    Sender {
        #[command(subcommand)]
        command: Sender,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum Receiver {
    Code {
        #[command(flatten)]
        account_arg: AccountArg,
        #[command(flatten)]
        address_types: AddressTypesArg,
    },
    Decode {
        notification: String,
        #[command(flatten)]
        account_arg: AccountArg,
        #[command(flatten)]
        address_types: AddressTypesArg,
        #[arg(short = 'i', default_value_t = 0)]
        start_address_index: u64,
        #[arg(short = 'f')]
        last_address_index: Option<u64>,
        #[arg(short = 'P', default_value_t = false)]
        show_private_key: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum Sender {
    Address {
        #[command(flatten)]
        account_arg: AccountArg,
        #[arg(short = 'r')]
        recipient_index: u32,
        #[command(flatten)]
        address_type: AddressTypeArg,
        recipient_payment_code: String,
        #[arg(short = 'i', default_value_t = 0)]
        start_address_index: u64,
        #[arg(short = 'f')]
        last_address_index: Option<u64>,
    },
    Notify {
        #[command(flatten)]
        account_arg: AccountArg,
        #[arg(short = 'r')]
        recipient_index: u32,
        #[command(flatten)]
        address_type: AddressTypeArg,
        recipient_payment_code: String,
    },
}

#[derive(Debug, Clone, Args)]
struct AccountArg {
    #[arg(short, long, default_value_t = 0)]
    account: u32,
}

#[derive(Debug, Clone, Args)]
struct AddressTypeArg {
    #[arg(short = 't', value_enum, rename_all = "lower", default_value_t = AddressType::default())]
    address_type: AddressType,
}

#[derive(Debug, Clone, Args)]
struct AddressTypesArg {
    #[arg(short = 't', rename_all = "lower", default_values_t = vec![AddressType::default()])]
    address_types: Vec<AddressType>,
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
                account_arg: AccountArg { account },
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

                Ok(format!("{}", recipient.payment_code()).into())
            }
            Receiver::Decode {
                notification,
                account_arg: AccountArg { account },
                address_types: AddressTypesArg { address_types },
                start_address_index,
                last_address_index,
                show_private_key,
            } => {
                let script = Script::from_hex(&notification)?;

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

                    let mut lines: Vec<String> = Vec::with_capacity(
                        range.end().saturating_add(1).saturating_sub(*range.start()) as usize,
                    );
                    for i in range {
                        let (address, public_key, private_key) =
                            recipient.key_info(&secp, &commitment, i)?;
                        if show_private_key {
                            lines.push(format!("{i}: {address} {public_key} {private_key}"));
                        } else {
                            lines.push(format!("{i}: {address}"));
                        }
                    }

                    return Ok(Output::Plain(lines.join("\n")));
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
            Sender::Address {
                account_arg,
                recipient_index,
                address_type: AddressTypeArg { address_type },
                recipient_payment_code,
                start_address_index,
                last_address_index,
            } => {
                let recipient = bip351::PaymentCode::from_str(&recipient_payment_code)?;

                let seed = get_seed_hex()?;

                let AccountArg { account } = account_arg;
                let sender = bip351::Sender::from_seed(&secp, &seed, Network::Bitcoin, account)?;

                let (_, commitment) =
                    sender.notify(&secp, &recipient, recipient_index, address_type.into())?;

                let range = index_range(start_address_index, last_address_index);

                let mut lines: Vec<String> = Vec::with_capacity(
                    range.end().saturating_add(1).saturating_sub(*range.start()) as usize,
                );
                for i in range {
                    let address = sender.address(&secp, &commitment, i)?;
                    lines.push(format!("{i}: {address}"));
                }

                Ok(Output::Plain(lines.join("\n")))
            }
            Sender::Notify {
                account_arg,
                recipient_index,
                address_type: AddressTypeArg { address_type },
                recipient_payment_code,
            } => {
                let recipient = bip351::PaymentCode::from_str(&recipient_payment_code)?;

                let seed = get_seed_hex()?;

                let AccountArg { account } = account_arg;
                let sender = bip351::Sender::from_seed(&secp, &seed, Network::Bitcoin, account)?;

                let (txout, _) =
                    sender.notify(&secp, &recipient, recipient_index, address_type.into())?;

                Ok(txout.script_pubkey.as_bytes().to_hex().into())
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
    Plain(String),
}

impl Output {
    fn print(&self) {
        match self {
            Self::Empty => {}
            Self::Plain(s) => println!("{s}"),
        }
    }
}

impl From<String> for Output {
    fn from(s: String) -> Self {
        Output::Plain(s)
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
