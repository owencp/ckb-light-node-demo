use super::{ChainStore, HeaderProviderWrapper, HeaderVerifier};
use crate::store::Store;

use ckb_chain_spec::consensus::Consensus;
use ckb_logger::{debug, info};
use ckb_network::{bytes::Bytes, CKBProtocolContext, CKBProtocolHandler, PeerIndex};
use ckb_types::{packed, prelude::*};
use crossbeam_channel::Receiver;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

const BAD_MESSAGE_BAN_TIME: Duration = Duration::from_secs(5 * 60);
const SEND_GET_GCS_FILTER_TOKEN: u64 = 0;
const SEND_GET_GCS_FILTER_HASHES_TOKEN: u64 = 1;
const SEND_GET_GCS_CHECKPOINT_TOKEN: u64 = 2;
const CONTROL_RECEIVER_TOKEN: u64 = 3;

const FILTER_RAW_DATA_SIZE: usize = 128;
const FILTER_NUM_HASHES: u8 = 10;

const MAX_FILTER_RANGE_SIZE: usize = 2000;
const MIN_CHECK_POINT_INTERVAL: u32 = 200_000;

pub struct FilterProtocol<S> {
    store: ChainStore<S>,
    consensus: Consensus,
    control_receiver: Receiver<ControlMessage>,
    peer_filter_hash_seed: Option<(PeerIndex, Option<u32>)>,
    pending_get_filtered_blocks: HashSet<packed::Byte32>,
}

impl<S> FilterProtocol<S> {
    pub fn new(
        store: ChainStore<S>,
        consensus: Consensus,
        control_receiver: Receiver<ControlMessage>,
    ) -> Self {
        Self {
            store,
            consensus,
            control_receiver,
            peer_filter_hash_seed: None,
            pending_get_filtered_blocks: HashSet::new(),
        }
    }
}

pub enum ControlMessage {
    SendTransaction(packed::Transaction),
}

pub enum GetGcsFilterMessage {
    GetGcsFilters(packed::Uint64, packed::Byte32),
    GetGcsFilterHashes(packed::Uint64, packed::Byte32),
    GetGcsFilterCheckPoint(packed::Byte32, packed::Uint32),
}

impl<S: Store + Send + Sync> CKBProtocolHandler for FilterProtocol<S> {
    fn init(&mut self, nc: Arc<dyn CKBProtocolContext + Sync>) {
        //TODO::    (Duration::from_millis(10) ???
        nc.set_notify(Duration::from_millis(10), SEND_GET_GCS_FILTER_TOKEN)
            .expect("set_notify should be ok");
        nc.set_notify(Duration::from_millis(10), SEND_GET_GCS_FILTER_HASHES_TOKEN)
            .expect("set_notify should be ok");
        nc.set_notify(Duration::from_millis(10), SEND_GET_GCS_CHECKPOINT_TOKEN)
            .expect("set_notify should be ok");
        nc.set_notify(Duration::from_millis(100), CONTROL_RECEIVER_TOKEN)
            .expect("set_notify should be ok");
    }

    fn notify(&mut self, nc: Arc<dyn CKBProtocolContext + Sync>, token: u64) {
        match token {
            SEND_GET_GCS_FILTER_TOKEN => {
                //TODO::
                //获取需要从full node同步的 gcs-filter： start_number, stop_hash
                //send message
                if let Some((peer, _)) = self.peer_filter_hash_seed {
                    let message = packed::GcsFilterMessage::new_builder()
                                  .set(
                                      packed::GetGcsFilters::new_builder()
                                      .start_number(start_block.pack())
                                      .stop_hash(stop_hash.clone())
                                      .build(),
                                    )
                                    .build();
                    if let Err(err) = nc.send_message_to(peer, message.as_bytes()) {
                        debug!("GcsFilterProtocol send GetGcsFilters error: {:?}", err);
                    }
                }
            }
            SEND_GET_GCS_FILTER_HASHES_TOKEN => {
                //TODO::
                //获取需要从full node同步的 gcs-filter： start_number, stop_hash
                //send message
                if let Some((peer, _)) = self.peer_filter_hash_seed {
                    let message = packed::GcsFilterMessage::new_builder()
                                  .set(
                                      packed::GetGcsFilterHashes::new_builder()
                                      .start_number(start_block.pack())
                                      .stop_hash(stop_hash.clone())
                                      .build(),
                                  )
                                  .build();
                    if let Err(err) = nc.send_message_to(peer, message.as_bytes()) {
                        debug!("GcsFilterProtocol send GetGcsFilterHashes error: {:?}", err);
                    }
                }
            }
            SEND_GET_GCS_CHECKPOINT_TOKEN => {
                //TODO::
                //获取stop_hash ， interval
                //send message
                if let Some((peer, _)) = self.peer_filter_hash_seed {
                    let message = packed::GcsFilterMessage::new_builder()
                                  .set(
                                      packed::GetGcsFilterCheckPoint::new_builder()
                                      .stop_hash(stop_hash.clone())
                                      .interval(interval.pack())
                                      .build(),
                                  )
                                  .build();
                    if let Err(err) = nc.send_message_to(peer, message.as_bytes()) {
                        debug!("GcsFilterProtocol send GetGcsFilterCheckPoint error: {:?}", err);
                    }
                }
            }
            CONTROL_RECEIVER_TOKEN => {
                if let Ok(msg) = self.control_receiver.try_recv() {
                    match msg {
                        ControlMessage::SendTransaction(transaction) => {
                            if let Some((peer, _)) = self.peer_filter_hash_seed {
                                let message = packed::GcsFilterMessage::new_builder()
                                    .set(
                                        packed::SendTransaction::new_builder()
                                            .transaction(transaction)
                                            .build(),
                                    )
                                    .build();
                                if let Err(err) = nc.send_message_to(peer, message.as_bytes()) {
                                    debug!("GcsFilterProtocol send SendTransaction error: {:?}", err);
                                }
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn connected(
        &mut self,
        _nc: Arc<dyn CKBProtocolContext + Sync>,
        peer: PeerIndex,
        _version: &str,
    ) {
        if self.peer_filter_hash_seed.is_none() {
            self.peer_filter_hash_seed = Some((peer, None));
        }
    }

    fn received(&mut self, nc: Arc<dyn CKBProtocolContext + Sync>, peer: PeerIndex, data: Bytes) {
        let message = match packed::GcsFilterMessage::from_slice(&data) {
            Ok(msg) => msg.to_enum(),
            _ => {
                info!("peer {} sends us a malformed Gcsfilter message", peer);
                nc.ban_peer(
                    peer,
                    BAD_MESSAGE_BAN_TIME,
                    String::from("send us a malformed Gcsfilter message"),
                );
                return;
            }
        };

        match message.as_reader() {
            packed::GcsFilterMessageUnionReader::GcsFilter(reader) => {
                //Vec<GcsFilter>  or GcsFilter??
                let gcs_filters = reader.to_entity();
                info!("received GcsFilters from peer: {}",peer);
                for gcsFilter in gcs_filters
                    .into_iter())
                {
                    //TODO::
                }
                //self.store
            }
            packed::GcsFilterMessageUnionReader::GcsFilterHashes(reader) => {
                let filter_hashes = reader.to_entity();
                //TODO::
                //self.store
            }
            packed::GcsFilterMessageUnionReader::GcsFilterCheckPoint(reader) => {
                //TODO::
                let filter_checkpoint = reader.to_entity();
                //self.store
            }
            _ => {
                // ignore
            }
        }
    }

    fn disconnected(&mut self, _nc: Arc<dyn CKBProtocolContext + Sync>, _peer: PeerIndex) {
        self.peer_filter_hash_seed = None;
        self.pending_get_filtered_blocks.clear();
    }
}
