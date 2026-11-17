//! v0.8.1 burn-down (#130-#137): one section per issue. Nothing here touches the
//! network beyond 127.0.0.1 or needs a database or a config.yaml.
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------------- #134: one ingestion cycle at a time ----------------

/// A stand-in cycle: counts how many run at once and how many ran, and takes long
/// enough that an unserialised second cycle would overlap it.
async fn fake_cycle(running: Arc<AtomicUsize>, peak: Arc<AtomicUsize>, ran: Arc<AtomicUsize>) -> Result<()> {
    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
    peak.fetch_max(now, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    running.fetch_sub(1, Ordering::SeqCst);
    ran.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[tokio::test]
async fn test_scheduled_and_requested_cycles_never_interleave() {
    let gate = Arc::new(CycleGate::default());
    let (running, peak, ran) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let mut tasks = Vec::new();
    for i in 0..6 {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        tasks.push(tokio::spawn(async move {
            if i % 2 == 0 {
                g.run(|| fake_cycle(r, p, n)).await
            } else {
                g.run_or_join(|| fake_cycle(r, p, n)).await
            }
        }));
    }
    for t in tasks {
        t.await.unwrap().unwrap();
    }
    assert_eq!(peak.load(Ordering::SeqCst), 1, "two cycles ran at the same time");
    assert!(ran.load(Ordering::SeqCst) >= 3, "every scheduled cycle runs");
}

#[tokio::test]
async fn test_requests_behind_a_running_cycle_share_one_new_cycle() {
    let gate = Arc::new(CycleGate::default());
    let (running, peak, ran) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    // A scheduled cycle is in flight.
    let first = {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        tokio::spawn(async move { g.run(|| fake_cycle(r, p, n)).await })
    };
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    // Five requests arrive while it runs: none may reuse it (it read the plan before
    // they arrived), and they need no more than one new cycle between them.
    let mut requests = Vec::new();
    for _ in 0..5 {
        let (g, r, p, n) = (gate.clone(), running.clone(), peak.clone(), ran.clone());
        requests.push(tokio::spawn(async move { g.run_or_join(|| fake_cycle(r, p, n)).await }));
    }
    first.await.unwrap().unwrap();
    for r in requests {
        r.await.unwrap().unwrap();
    }
    assert_eq!(ran.load(Ordering::SeqCst), 2, "the cycle in flight plus one for every waiting request");
    assert_eq!(peak.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_a_joined_request_gets_the_shared_cycles_error() {
    let gate = Arc::new(CycleGate::default());
    let slow_fail = || async {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        Err::<(), _>(anyhow::anyhow!("RPC read failed"))
    };
    let g = gate.clone();
    let inflight = tokio::spawn(async move { g.run(|| async { tokio::time::sleep(std::time::Duration::from_millis(30)).await; Ok(()) }).await });
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let (g1, g2) = (gate.clone(), gate.clone());
    let a = tokio::spawn(async move { g1.run_or_join(slow_fail).await });
    let b = tokio::spawn(async move { g2.run_or_join(slow_fail).await });
    inflight.await.unwrap().unwrap();
    let (a, b) = (a.await.unwrap(), b.await.unwrap());
    assert_eq!(a.unwrap_err().to_string(), "RPC read failed");
    assert_eq!(b.unwrap_err().to_string(), "RPC read failed");
}

#[tokio::test]
async fn test_a_dropped_request_does_not_cancel_its_cycle() {
    // The handler spawns the cycle and awaits its JoinHandle; dropping the handler's
    // future (a client gone, nginx's 60 s timeout) must leave the cycle running.
    let gate = Arc::new(CycleGate::default());
    let finished = Arc::new(AtomicUsize::new(0));
    let (g, f) = (gate.clone(), finished.clone());
    let request = async move {
        let cycle = tokio::spawn(async move {
            g.run_or_join(|| async move {
                tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                f.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
            .await
        });
        cycle.await
    };
    // The request is abandoned after 5 ms.
    assert!(tokio::time::timeout(std::time::Duration::from_millis(5), request).await.is_err());
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    assert_eq!(finished.load(Ordering::SeqCst), 1, "the cycle ran to the end");
}
