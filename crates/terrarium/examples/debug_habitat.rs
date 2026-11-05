use terrarium::contract::Run;
use terrarium::mapping::Behaviors;
use terrarium::simulator::{self};

fn main() {
    let run: Run = serde_json::from_str(
        r#"{"dagr":3,"run":{"id":"demo","title":"Demo"},"generated_at":"2026-09-03T15:00:00Z","projects":[{"id":"triage","title":"Triage Burrow"},{"id":"guard","title":"Guard Reef"}],"tasks":[{"id":"T1","title":"test","kind":"review","owner":"maintainer","project":"triage","state":"review","deps":[],"note":"test","attempts":[{"id":"T1·a1","n":1,"actor":"agent","state":"done","started_at":"2026-09-01T00:00:30Z","ended_at":"2026-09-01T00:01:00Z","outcome":{"result":"done","evidence":"reported","receipt":"exit 0"}}]},{"id":"T2","title":"lockfile","kind":"review","owner":"maintainer","project":"triage","state":"review","deps":[],"note":"test","attempts":[{"id":"T2·a1","n":1,"actor":"agent","state":"failed","started_at":"2026-09-01T00:02:00Z","ended_at":"2026-09-01T00:02:10Z","outcome":{"result":"failed","evidence":"reported","reason":"exit 0"}}]},{"id":"G1","title":"rm target","kind":"question","owner":"user","project":"guard","state":"blocked","deps":[],"unblock":"user","note":"test","attempts":[]},{"id":"THEORY","title":"docs","kind":"docs","owner":"maintainer","state":"queued","deps":[],"note":"test","attempts":[]}],"events":[{"at":"2026-09-01T00:01:00Z","type":"attempt_settled","attempt":"T1·a1","detail":"settled"}]}"#,
    )
    .unwrap();

    let taste = Behaviors::default();
    let habitat = simulator::simulate(&run, &taste);

    println!("Habitat zones: {:?}", habitat.zones);
    println!("Habitat crabs: {} crabs", habitat.crabs.len());
    for crab in &habitat.crabs {
        println!(
            "  - {} (zone={}, xy={:?})",
            crab.id, crab.zone, crab.zone_xy
        );
    }
}
