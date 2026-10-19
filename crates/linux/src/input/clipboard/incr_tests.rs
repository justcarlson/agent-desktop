use super::*;

fn drain(mut sender: IncrSender) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    while let Some(chunk) = sender.next_chunk() {
        chunks.push(chunk);
    }
    chunks
}

#[test]
fn an_exact_multiple_ends_with_a_zero_chunk() {
    let chunks = drain(IncrSender::new(vec![1; 8], 4));
    let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
    assert_eq!(sizes, [4, 4, 0]);
}

#[test]
fn a_remainder_emits_one_short_chunk_then_the_zero_chunk() {
    let chunks = drain(IncrSender::new(vec![1; 10], 4));
    let sizes: Vec<usize> = chunks.iter().map(Vec::len).collect();
    assert_eq!(sizes, [4, 4, 2, 0]);
}

#[test]
fn an_empty_payload_emits_only_the_zero_chunk() {
    assert_eq!(drain(IncrSender::new(Vec::new(), 4)), [Vec::<u8>::new()]);
}

#[test]
fn the_sender_stays_finished() {
    let mut sender = IncrSender::new(vec![1], 4);
    assert!(sender.next_chunk().is_some());
    assert_eq!(sender.next_chunk(), Some(Vec::new()));
    assert_eq!(sender.next_chunk(), None);
    assert_eq!(sender.next_chunk(), None);
}

#[test]
fn the_receiver_reassembles_what_the_sender_chunked() {
    let data: Vec<u8> = (0..=255).cycle().take(1000).collect();
    let mut receiver = IncrReceiver::new(4096);
    for chunk in drain(IncrSender::new(data.clone(), 64)) {
        receiver.push(&chunk).expect("within the cap");
    }
    assert_eq!(receiver.into_bytes(), Some(data));
}

#[test]
fn a_transfer_without_its_zero_chunk_is_not_a_value() {
    let mut receiver = IncrReceiver::new(4096);
    assert_eq!(receiver.push(&[1, 2, 3]).expect("chunk"), Push::More);
    assert_eq!(receiver.into_bytes(), None);
}

#[test]
fn a_chunk_that_passes_the_cap_aborts() {
    let mut receiver = IncrReceiver::new(5);
    receiver.push(&[0; 4]).expect("under the cap");
    let error = receiver.push(&[0; 2]).expect_err("over the cap");
    assert_eq!(error.code, ErrorCode::ActionFailed);
}
