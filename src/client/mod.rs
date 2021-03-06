use crossbeam_channel;
use error::{ClientError, ConnectError};
use futures::{sync::mpsc, Future, Sink};
use mqtt3::{PacketIdentifier, Publish, QoS, Subscribe, SubscribeTopic};
use std::sync::Arc;
use MqttOptions;

pub mod connection;
pub mod mqttasync;
pub mod mqttstate;
pub mod network;
pub mod prepend;

#[derive(Debug)]
pub enum Notification {
    Publish(Publish),
    PubAck(PacketIdentifier),
    PubRec(PacketIdentifier),
    PubRel(PacketIdentifier),
    PubComp(PacketIdentifier),
    SubAck(PacketIdentifier),
    None,
}

/// Requests to network event loop
#[derive(Debug)]
pub enum Request {
    Publish(Publish),
    Subscribe(Subscribe),
    PubAck(PacketIdentifier),
    PubRec(PacketIdentifier),
    PubRel(PacketIdentifier),
    PubComp(PacketIdentifier),
    Ping,
    Reconnect(MqttOptions),
    Disconnect,
    None,
}

pub struct UserHandle {
    request_tx: mpsc::Sender<Request>,
    notification_rx: crossbeam_channel::Receiver<Notification>,
}

pub struct MqttClient {
    request_tx: mpsc::Sender<Request>,
    max_packet_size: usize,
}

impl MqttClient {
    pub fn start(opts: MqttOptions) -> Result<(Self, crossbeam_channel::Receiver<Notification>), ConnectError> {
        let UserHandle { request_tx, notification_rx } = connection::Connection::run(opts)?;

        //TODO: Remove max packet size hardcode
        let client = MqttClient { request_tx,
                                  max_packet_size: 1000 };

        Ok((client, notification_rx))
    }

    pub fn publish<S: Into<String>, V: Into<Vec<u8>>>(&mut self, topic: S, qos: QoS, payload: V) -> Result<(), ClientError> {
        let payload = payload.into();
        if payload.len() > self.max_packet_size {
            return Err(ClientError::PacketSizeLimitExceeded);
        }

        //TODO: Rename `pid` to `pkid` in mqtt311
        let publish = Publish { dup: false,
                                qos,
                                retain: false,
                                topic_name: topic.into(),
                                pid: None,
                                payload: Arc::new(payload) };

        let tx = &mut self.request_tx;
        tx.send(Request::Publish(publish)).wait()?;
        Ok(())
    }

    pub fn subscribe<S: Into<String>>(&mut self, topic: S, qos: QoS) -> Result<(), ClientError> {
        let topic = SubscribeTopic { topic_path: topic.into(),
                                     qos: qos };
        let subscribe = Subscribe { pid: PacketIdentifier::zero(),
                                    topics: vec![topic] };

        let tx = &mut self.request_tx;
        tx.send(Request::Subscribe(subscribe)).wait()?;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), ClientError> {
        let tx = &mut self.request_tx;
        tx.send(Request::Disconnect).wait()?;
        Ok(())
    }
}
