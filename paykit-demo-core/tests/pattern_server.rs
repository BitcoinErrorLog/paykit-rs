use anyhow::Result;
use paykit_demo_core::{
    AcceptedConnection, Identity, NoisePattern, NoiseRawClientHelper, NoiseServerHelper,
};
use paykit_interactive::{PaykitNoiseChannel, PaykitNoiseMessage};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn roundtrip(pattern: NoisePattern) -> Result<()> {
    let server_identity = Identity::generate();
    let device_label = format!("pattern-server-{}", pattern.negotiation_byte());
    let device_bytes = device_label.into_bytes();

    let server = NoiseServerHelper::create_server(&server_identity, &device_bytes);
    let server_pk = NoiseServerHelper::get_static_public_key(&server_identity, &device_bytes);
    let server_sk = NoiseServerHelper::derive_x25519_key(&server_identity, &device_bytes);

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let host = addr.to_string();

    let server_task = tokio::spawn({
        let server = Arc::clone(&server);
        async move {
            let (mut stream, _) = listener.accept().await?;

            let mut pattern_byte = [0u8; 1];
            stream.read_exact(&mut pattern_byte).await?;
            let requested = NoisePattern::try_from(pattern_byte[0])?;
            assert_eq!(requested, pattern);

            let conn =
                NoiseServerHelper::accept_with_pattern(server, &server_sk, stream, requested)
                    .await?;

            match (requested, conn) {
                (NoisePattern::IKRaw, AcceptedConnection::IKRaw { mut channel, .. }) => {
                    let msg = channel.recv().await?;
                    assert!(matches!(msg, PaykitNoiseMessage::Ack));
                    channel.send(PaykitNoiseMessage::Ack).await?;
                }
                (NoisePattern::N, AcceptedConnection::N { mut channel }) => {
                    let msg = channel.recv().await?;
                    assert!(matches!(msg, PaykitNoiseMessage::Ack));
                    // N is one-way: server does not send encrypted data back.
                }
                (NoisePattern::NN, AcceptedConnection::NN { mut channel, .. }) => {
                    let msg = channel.recv().await?;
                    assert!(matches!(msg, PaykitNoiseMessage::Ack));
                    channel.send(PaykitNoiseMessage::Ack).await?;
                }
                _ => anyhow::bail!("Unexpected connection variant for {:?}", requested),
            }

            Ok::<_, anyhow::Error>(())
        }
    });

    let client_task = tokio::spawn(async move {
        let client_identity = Identity::generate();

        let mut channel = match pattern {
            NoisePattern::IKRaw => {
                let ctx = format!("pattern-client-{}", client_identity.public_key());
                let x25519_sk = NoiseRawClientHelper::derive_x25519_key(
                    &client_identity.keypair.secret_key(),
                    ctx.as_bytes(),
                );
                NoiseRawClientHelper::connect_ik_raw_with_negotiation(&x25519_sk, &host, &server_pk)
                    .await?
            }
            NoisePattern::N => {
                NoiseRawClientHelper::connect_anonymous_with_negotiation(&host, &server_pk).await?
            }
            NoisePattern::NN => {
                let (channel, _) =
                    NoiseRawClientHelper::connect_ephemeral_with_negotiation(&host).await?;
                channel
            }
            NoisePattern::IK | NoisePattern::XX => {
                unreachable!("pattern-aware tests cover IK-raw, N, and NN only")
            }
        };

        channel.send(PaykitNoiseMessage::Ack).await?;
        if pattern != NoisePattern::N {
            let reply = channel.recv().await?;
            assert!(matches!(reply, PaykitNoiseMessage::Ack));
        }

        Ok::<_, anyhow::Error>(())
    });

    server_task.await??;
    client_task.await??;
    Ok(())
}

#[tokio::test]
async fn test_pattern_server_ik_raw_roundtrip() -> Result<()> {
    roundtrip(NoisePattern::IKRaw).await
}

#[tokio::test]
async fn test_pattern_server_n_roundtrip() -> Result<()> {
    roundtrip(NoisePattern::N).await
}

#[tokio::test]
async fn test_pattern_server_nn_roundtrip() -> Result<()> {
    roundtrip(NoisePattern::NN).await
}
