// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod crypto;

use status_code::StatusCode;
use cita_cloud_proto::blockchain::{RawTransaction, RawTransactions, Block, CompactBlock, CompactBlockBody};
use cita_cloud_proto::blockchain::raw_transaction::Tx;
use cita_cloud_proto::common::Address;
use cita_cloud_proto::kms::kms_service_client::KmsServiceClient;
use cita_cloud_proto::kms::HashDataRequest;

pub fn unix_now() -> u64 {
    let d = ::std::time::UNIX_EPOCH.elapsed().unwrap();
    d.as_secs() * 1_000 + u64::from(d.subsec_millis())
}

pub fn clean_0x(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}

pub fn h160_address_check(address: Option<&Address>) -> Result<(), StatusCode> {
    match address {
        Some(addr) => {
            if addr.address.len() == 20 {
                Ok(())
            } else {
                Err(StatusCode::ProvideAddressError)
            }
        }
        None => Err(StatusCode::NoProvideAddress),
    }
}

pub fn get_tx_hash(raw_tx: &RawTransaction) -> Result<&[u8], StatusCode> {
    match raw_tx.tx {
        Some(Tx::NormalTx(ref normal_tx)) => Ok(&normal_tx.transaction_hash),
        Some(Tx::UtxoTx(ref utxo_tx)) => Ok(&utxo_tx.transaction_hash),
        None => return Err(StatusCode::NoTransaction),
    }
}

pub fn get_tx_hash_list(raw_txs: &RawTransactions) -> Result<Vec<Vec<u8>>, StatusCode> {
    let mut hashes = Vec::new();
    for raw_tx in &raw_txs.body {
        hashes.push(get_tx_hash(raw_tx)?.to_vec())
    }
    Ok(hashes)
}

pub fn extract_compact(block: Block) -> CompactBlock {
    let mut compact_body = CompactBlockBody { tx_hashes: vec![] };

    if let Some(body) = block.body {
        for raw_tx in body.body {
            match raw_tx.tx {
                Some(Tx::NormalTx(normal_tx)) => {
                    compact_body.tx_hashes.push(normal_tx.transaction_hash)
                }
                Some(Tx::UtxoTx(utxo_tx)) => compact_body.tx_hashes.push(utxo_tx.transaction_hash),
                None => {}
            }
        }
    }

    CompactBlock {
        version: block.version,
        header: block.header,
        body: Some(compact_body),
    }
}

