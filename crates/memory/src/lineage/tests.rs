use chrono::Utc;

use super::*;

#[test]
fn test_lineage_id_generation() {
    let id1 = LineageId::generate();
    let id2 = LineageId::generate();
    assert_ne!(id1, id2);
    assert!(id1.as_str().starts_with("ln-"));
}

#[test]
fn test_source_ref_display() {
    let worker = SourceRef::worker("w1").with_role("coder");
    assert_eq!(worker.to_string(), "worker:w1 (coder)");

    let human = SourceRef::human("kev");
    assert_eq!(human.to_string(), "human:kev");
}

#[test]
fn test_proposal_status_display() {
    assert_eq!(ProposalStatus::Pending.to_string(), "pending");
    assert_eq!(ProposalStatus::Accepted.to_string(), "accepted");
    let rejected = ProposalStatus::Rejected {
        reason: "too risky".into(),
    };
    assert!(rejected.to_string().starts_with("rejected:"));
}

#[test]
fn test_edge_type_roundtrip() {
    for t in [
        EdgeType::BasedOn,
        EdgeType::Proposes,
        EdgeType::Decides,
        EdgeType::Implements,
        EdgeType::Recognizes,
    ] {
        assert_eq!(EdgeType::parse(t.as_str()), Some(t));
    }
    assert_eq!(EdgeType::parse("unknown"), None);
}

#[test]
fn test_insert_and_retrieve_node() {
    let store = test_store();
    let obs = LineageNode::Observation {
        id: LineageId::generate(),
        source: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Found performance bottleneck".into(),
        context: Some("wp-gpu-node".into()),
    };

    store.insert_node(&obs).unwrap();
    let retrieved = store
        .get_node(obs.id())
        .unwrap()
        .expect("node should exist");

    assert_eq!(retrieved.id(), obs.id());
    assert_eq!(retrieved.node_type(), "observation");
}

#[test]
fn test_insert_edge_and_traverse() {
    let store = test_store();

    let obs = LineageNode::Observation {
        id: LineageId::generate(),
        source: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Observed issue".into(),
        context: None,
    };
    let prop = LineageNode::Proposal {
        id: LineageId::generate(),
        based_on: vec![obs.id().clone()],
        proposer: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Propose fix".into(),
        status: ProposalStatus::Pending,
    };

    store.insert_node(&obs).unwrap();
    store.insert_node(&prop).unwrap();

    let edge = LineageEdge {
        from_id: obs.id().clone(),
        to_id: prop.id().clone(),
        edge_type: EdgeType::BasedOn,
        weight: 1.0,
        created_at: Utc::now(),
    };
    store.insert_edge(&edge).unwrap();

    let outgoing = store.outgoing_edges(obs.id()).unwrap();
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].to_id, *prop.id());

    let incoming = store.incoming_edges(prop.id()).unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].from_id, *obs.id());
}

#[test]
fn test_trace_forward() {
    let store = test_store();
    let query = SqlProvenanceQuery::new(store);

    // Build chain: observation -> proposal -> decision
    let obs = LineageNode::Observation {
        id: LineageId::generate(),
        source: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Obs".into(),
        context: None,
    };
    let prop = LineageNode::Proposal {
        id: LineageId::generate(),
        based_on: vec![obs.id().clone()],
        proposer: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Prop".into(),
        status: ProposalStatus::Pending,
    };
    let dec = LineageNode::Decision {
        id: LineageId::generate(),
        proposal_id: prop.id().clone(),
        decision_maker: SourceRef::human("kev"),
        timestamp: Utc::now(),
        outcome: DecisionOutcome::Approved,
        rationale: "Looks good".into(),
    };

    for node in [&obs, &prop, &dec] {
        query.store.insert_node(node).unwrap();
    }

    query
        .store
        .insert_edge(&LineageEdge {
            from_id: obs.id().clone(),
            to_id: prop.id().clone(),
            edge_type: EdgeType::BasedOn,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();
    query
        .store
        .insert_edge(&LineageEdge {
            from_id: prop.id().clone(),
            to_id: dec.id().clone(),
            edge_type: EdgeType::Decides,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();

    let forward = query.trace_forward(obs.id()).unwrap();
    assert_eq!(forward.len(), 2);
}

#[test]
fn test_trace_backward() {
    let store = test_store();
    let query = SqlProvenanceQuery::new(store);

    let obs = LineageNode::Observation {
        id: LineageId::generate(),
        source: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Obs".into(),
        context: None,
    };
    let prop = LineageNode::Proposal {
        id: LineageId::generate(),
        based_on: vec![obs.id().clone()],
        proposer: SourceRef::worker("w1"),
        timestamp: Utc::now(),
        description: "Prop".into(),
        status: ProposalStatus::Pending,
    };

    query.store.insert_node(&obs).unwrap();
    query.store.insert_node(&prop).unwrap();
    query
        .store
        .insert_edge(&LineageEdge {
            from_id: obs.id().clone(),
            to_id: prop.id().clone(),
            edge_type: EdgeType::BasedOn,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();

    let backward = query.trace_backward(prop.id()).unwrap();
    assert_eq!(backward.len(), 1);
    assert_eq!(backward[0].id(), obs.id());
}

#[test]
fn test_recognition_callback() {
    let store = test_store();
    let query = SqlProvenanceQuery::new(store);
    let builder = RecognitionBuilder::new(query);

    // Build full chain: obs -> prop -> dec -> impl
    let obs = LineageNode::Observation {
        id: LineageId::generate(),
        source: SourceRef::worker("w1").with_role("coder"),
        timestamp: Utc::now(),
        description: "Found pattern".into(),
        context: None,
    };
    let prop = LineageNode::Proposal {
        id: LineageId::generate(),
        based_on: vec![obs.id().clone()],
        proposer: SourceRef::worker("w1").with_role("coder"),
        timestamp: Utc::now(),
        description: "Propose policy".into(),
        status: ProposalStatus::Accepted,
    };
    let dec = LineageNode::Decision {
        id: LineageId::generate(),
        proposal_id: prop.id().clone(),
        decision_maker: SourceRef::human("kev"),
        timestamp: Utc::now(),
        outcome: DecisionOutcome::Approved,
        rationale: "Approved".into(),
    };
    let impl_node = LineageNode::Implementation {
        id: LineageId::generate(),
        decision_id: dec.id().clone(),
        implementer: SourceRef::worker("w2"),
        timestamp: Utc::now(),
        artifacts: vec!["policy.yaml".into()],
    };

    for node in [&obs, &prop, &dec, &impl_node] {
        builder.query.store.insert_node(node).unwrap();
    }

    builder
        .query
        .store
        .insert_edge(&LineageEdge {
            from_id: obs.id().clone(),
            to_id: prop.id().clone(),
            edge_type: EdgeType::BasedOn,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();
    builder
        .query
        .store
        .insert_edge(&LineageEdge {
            from_id: prop.id().clone(),
            to_id: dec.id().clone(),
            edge_type: EdgeType::Decides,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();
    builder
        .query
        .store
        .insert_edge(&LineageEdge {
            from_id: dec.id().clone(),
            to_id: impl_node.id().clone(),
            edge_type: EdgeType::Implements,
            weight: 1.0,
            created_at: Utc::now(),
        })
        .unwrap();

    let callback = builder.build_callback(impl_node.id()).unwrap();
    assert!(callback.is_some());
    let cb = callback.unwrap();
    assert_eq!(cb.original_proposal_id, *prop.id());
    assert_eq!(cb.implementation_id, *impl_node.id());
}
