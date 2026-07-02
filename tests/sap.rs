//! Exercises the MTP3-User SAP end to end with an in-memory provider, and shows
//! the composable-router pattern: a provider that dispatches by destination and
//! is *itself* an `Mtp3UserPart`, so a user (SCCP) still sees one SAP.

use tokio::sync::mpsc;
use tokio::sync::Mutex;

use ss7_mtp3::{
    Mtp3Error, Mtp3Event, Mtp3Msu, Mtp3UserPart, NetworkIndicator, PointCode, ServiceIndicator,
    Variant,
};

/// A loopback provider: everything you `send` comes back as a `Transfer` event.
/// Stands in for a real MTP3/M3UA provider in tests.
struct Loopback {
    tx: mpsc::UnboundedSender<Mtp3Event>,
    rx: Mutex<mpsc::UnboundedReceiver<Mtp3Event>>,
    label: &'static str,
}

impl Loopback {
    fn new(label: &'static str) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Mutex::new(rx),
            label,
        }
    }
}

#[async_trait::async_trait]
impl Mtp3UserPart for Loopback {
    async fn send(&self, mut msu: Mtp3Msu) -> Result<(), Mtp3Error> {
        // Tag the payload so the test can tell which provider handled it.
        msu.data.push(self.label.as_bytes()[0]);
        self.tx
            .send(Mtp3Event::Transfer(msu))
            .map_err(|e| Mtp3Error::Transport(e.to_string()))
    }
    async fn recv(&self) -> Result<Mtp3Event, Mtp3Error> {
        self.rx
            .lock()
            .await
            .recv()
            .await
            .ok_or(Mtp3Error::OutOfService)
    }
    fn is_available(&self, _dpc: PointCode) -> bool {
        true
    }
}

fn msu_to(dpc: u32, data: Vec<u8>) -> Mtp3Msu {
    Mtp3Msu {
        si: ServiceIndicator::SCCP,
        ni: NetworkIndicator::International,
        mp: 0,
        opc: PointCode::from_value(1, Variant::Itu).unwrap(),
        dpc: PointCode::from_value(dpc, Variant::Itu).unwrap(),
        sls: 0,
        data,
    }
}

#[tokio::test]
async fn send_recv_round_trip() {
    let p = Loopback::new("A");
    p.send(msu_to(100, b"hello".to_vec())).await.unwrap();
    match p.recv().await.unwrap() {
        Mtp3Event::Transfer(m) => {
            assert_eq!(m.dpc.value(), 100);
            assert_eq!(&m.data, b"helloA"); // tagged by provider A
        }
        other => panic!("expected Transfer, got {other:?}"),
    }
}

/// A router that dispatches by DPC across several providers, and is itself an
/// `Mtp3UserPart` — the "collapse into one SAP" the STP needs.
struct Router {
    providers: Vec<Box<dyn Mtp3UserPart>>,
    route: fn(PointCode) -> usize,
}

#[async_trait::async_trait]
impl Mtp3UserPart for Router {
    async fn send(&self, msu: Mtp3Msu) -> Result<(), Mtp3Error> {
        let idx = (self.route)(msu.dpc);
        self.providers
            .get(idx)
            .ok_or(Mtp3Error::Unreachable(msu.dpc))?
            .send(msu)
            .await
    }
    async fn recv(&self) -> Result<Mtp3Event, Mtp3Error> {
        Err(Mtp3Error::OutOfService) // (real impl would select across providers)
    }
    fn is_available(&self, dpc: PointCode) -> bool {
        self.providers
            .get((self.route)(dpc))
            .is_some_and(|p| p.is_available(dpc))
    }
}

#[tokio::test]
async fn router_dispatches_by_destination() {
    // Even destinations → provider A, odd → provider B.
    let router = Router {
        providers: vec![Box::new(Loopback::new("A")), Box::new(Loopback::new("B"))],
        route: |dpc| (dpc.value() % 2) as usize,
    };

    // SCCP just holds a `dyn Mtp3UserPart` — it doesn't know there are two.
    let sap: &dyn Mtp3UserPart = &router;
    sap.send(msu_to(2, b"x".to_vec())).await.unwrap(); // even → A
    sap.send(msu_to(3, b"y".to_vec())).await.unwrap(); // odd  → B

    assert!(sap.is_available(PointCode::from_value(2, Variant::Itu).unwrap()));

    // Drain each underlying provider to confirm the routing landed correctly.
    match router.providers[0].recv().await.unwrap() {
        Mtp3Event::Transfer(m) => assert_eq!(&m.data, b"xA"),
        _ => panic!("A"),
    }
    match router.providers[1].recv().await.unwrap() {
        Mtp3Event::Transfer(m) => assert_eq!(&m.data, b"yB"),
        _ => panic!("B"),
    }
}
