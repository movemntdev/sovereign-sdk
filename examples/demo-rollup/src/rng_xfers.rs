use std::env;
use std::future::Future;
use std::pin::Pin;

use borsh::ser::BorshSerialize;
use const_rollup_config::SEQUENCER_DA_ADDRESS;
use demo_stf::runtime::Runtime;
use jupiter::verifier::address::CelestiaAddress;
use sov_bank::call::CallMessage;
use sov_bank::Coins;
use sov_modules_api::default_context::DefaultContext;
use sov_modules_api::default_signature::private_key::DefaultPrivateKey;
use sov_modules_api::transaction::Transaction;
use sov_modules_api::{Address, AddressBech32, PublicKey, Spec};
use sov_rollup_interface::da::DaSpec;
use sov_rollup_interface::mocks::{TestBlob, TestBlock, TestBlockHeader, TestHash};
use sov_rollup_interface::services::da::DaService;

pub struct RngDaService;

fn generate_transfers(n: usize, start_nonce: u64) -> Vec<u8> {
    let sender_address =
        "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqmlyjd6".to_string();
    let token_name = "sov-test-token";
    let sa = Address::from(
        AddressBech32::try_from(sender_address)
            .unwrap_or_else(|_e| panic!("Failed generating transfers")),
    );
    let token_address = sov_bank::get_token_address::<DefaultContext>(token_name, sa.as_ref(), 11);
    let mut message_vec = vec![];
    for i in 1..(n + 1) {
        let priv_key = DefaultPrivateKey::generate();
        let address: <DefaultContext as Spec>::Address = priv_key.pub_key().to_address();
        let pk = DefaultPrivateKey::from_hex("236e80cb222c4ed0431b093b3ac53e6aa7a2273fe1f4351cd354989a823432a27b758bf2e7670fafaf6bf0015ce0ff5aa802306fc7e3f45762853ffc37180fe6").unwrap();
        let msg: sov_bank::call::CallMessage<DefaultContext> =
            CallMessage::<DefaultContext>::Transfer {
                to: address,
                coins: Coins {
                    amount: 1,
                    token_address: token_address.clone(),
                },
            };
        let enc_msg = Runtime::<DefaultContext>::encode_bank_call(msg);
        let tx =
            Transaction::<DefaultContext>::new_signed_tx(&pk, enc_msg, start_nonce + (i as u64));
        let ser_tx = tx.try_to_vec().unwrap();
        message_vec.push(ser_tx)
    }
    message_vec.try_to_vec().unwrap()
}

fn generate_create(start_nonce: u64) -> Vec<u8> {
    let sender_address =
        "sov15vspj48hpttzyvxu8kzq5klhvaczcpyxn6z6k0hwpwtzs4a6wkvqmlyjd6".to_string();
    let mut message_vec = vec![];

    let pk = DefaultPrivateKey::from_hex("236e80cb222c4ed0431b093b3ac53e6aa7a2273fe1f4351cd354989a823432a27b758bf2e7670fafaf6bf0015ce0ff5aa802306fc7e3f45762853ffc37180fe6").unwrap();
    let minter_address = Address::from(
        AddressBech32::try_from(sender_address)
            .unwrap_or_else(|_e| panic!("Failed generating token create transaction")),
    );
    let msg: sov_bank::call::CallMessage<DefaultContext> =
        CallMessage::<DefaultContext>::CreateToken {
            salt: 11,
            token_name: "sov-test-token".to_string(),
            initial_balance: 100000000,
            minter_address: minter_address.clone(),
            authorized_minters: vec![minter_address],
        };
    let enc_msg = Runtime::<DefaultContext>::encode_bank_call(msg);
    let tx = Transaction::<DefaultContext>::new_signed_tx(&pk, enc_msg, start_nonce);
    let ser_tx = tx.try_to_vec().unwrap();
    message_vec.push(ser_tx);
    message_vec.try_to_vec().unwrap()
}

impl RngDaService {
    pub fn new() -> Self {
        RngDaService
    }
}

impl Default for RngDaService {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RngDaSpec;

impl DaSpec for RngDaSpec {
    type SlotHash = TestHash;
    type BlockHeader = TestBlockHeader;
    type BlobTransaction = TestBlob<CelestiaAddress>;
    type InclusionMultiProof = [u8; 32];
    type CompletenessProof = ();
    type ChainParams = ();
}

impl DaService for RngDaService {
    type RuntimeConfig = ();
    type Spec = RngDaSpec;
    type FilteredBlock = TestBlock;
    type Future<T> = Pin<Box<dyn Future<Output = Result<T, Self::Error>> + Send>>;
    type Error = anyhow::Error;

    fn new(
        _config: Self::RuntimeConfig,
        _chain_params: <Self::Spec as DaSpec>::ChainParams,
    ) -> Self {
        RngDaService::new()
    }

    fn get_finalized_at(&self, height: u64) -> Self::Future<Self::FilteredBlock> {
        let num_bytes = height.to_le_bytes();
        let mut barray = [0u8; 32];
        barray[..num_bytes.len()].copy_from_slice(&num_bytes);

        let block = TestBlock {
            curr_hash: barray,
            header: TestBlockHeader {
                prev_hash: TestHash([0u8; 32]),
            },
            height,
        };

        Box::pin(async move { Ok(block) })
    }

    fn get_block_at(&self, _height: u64) -> Self::Future<Self::FilteredBlock> {
        unimplemented!()
    }

    fn extract_relevant_txs(
        &self,
        block: &Self::FilteredBlock,
    ) -> Vec<<Self::Spec as DaSpec>::BlobTransaction> {
        let mut num_txns = 10000;
        if let Ok(val) = env::var("TXNS_PER_BLOCK") {
            num_txns = val
                .parse()
                .expect("TXNS_PER_BLOCK var should be a +ve number");
        }

        let data = if block.height == 0 {
            // creating the token
            generate_create(0)
        } else {
            // generating the transfer transactions
            generate_transfers(num_txns, (block.height - 1) * (num_txns as u64))
        };

        let address = CelestiaAddress::try_from(&SEQUENCER_DA_ADDRESS[..]).unwrap();
        let blob = TestBlob::new(data, address, [0u8; 32]);

        vec![blob]
    }

    fn get_extraction_proof(
        &self,
        _block: &Self::FilteredBlock,
        _blobs: &[<Self::Spec as DaSpec>::BlobTransaction],
    ) -> (
        <Self::Spec as DaSpec>::InclusionMultiProof,
        <Self::Spec as DaSpec>::CompletenessProof,
    ) {
        unimplemented!()
    }

    fn send_transaction(&self, _blob: &[u8]) -> Self::Future<()> {
        unimplemented!()
    }
}
