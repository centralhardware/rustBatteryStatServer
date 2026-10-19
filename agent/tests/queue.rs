use battery_agent::queue::{Queue, SendError};
use battery_model::BatteryHealth;
use std::cell::RefCell;

fn sample(n: u32) -> BatteryHealth {
    BatteryHealth { device_id: format!("d{n}"), date_time: n, ..Default::default() }
}

fn temp_queue(name: &str) -> (Queue, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("battery-agent-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("queue.jsonl");
    (Queue::new(&path), path)
}

#[test]
fn flush_sends_in_order_and_empties() {
    let (q, path) = temp_queue("order");
    (1..=3).for_each(|n| q.push(&sample(n)));
    let sent = RefCell::new(vec![]);
    q.flush(|s| {
        sent.borrow_mut().push(s.date_time);
        Ok(())
    })
    .unwrap();
    assert_eq!(*sent.borrow(), vec![1, 2, 3]);
    assert!(q.is_empty());
    assert!(!path.exists());
}

#[test]
fn permanent_rejection_does_not_block_the_queue() {
    let (q, _) = temp_queue("permanent");
    (1..=3).for_each(|n| q.push(&sample(n)));
    let sent = RefCell::new(vec![]);
    q.flush(|s| {
        if s.date_time == 1 {
            return Err(SendError::Permanent("HTTP 422".into()));
        }
        sent.borrow_mut().push(s.date_time);
        Ok(())
    })
    .unwrap();
    assert_eq!(*sent.borrow(), vec![2, 3]);
    assert!(q.is_empty());
}

#[test]
fn retryable_failure_keeps_the_rest() {
    let (q, _) = temp_queue("retry");
    (1..=3).for_each(|n| q.push(&sample(n)));
    let r = q.flush(|s| if s.date_time == 2 { Err(SendError::Retry("down".into())) } else { Ok(()) });
    assert_eq!(r, Err("down".into()));
    let sent = RefCell::new(vec![]);
    q.flush(|s| {
        sent.borrow_mut().push(s.date_time);
        Ok(())
    })
    .unwrap();
    assert_eq!(*sent.borrow(), vec![2, 3]);
}

#[test]
fn unreadable_lines_are_dropped() {
    let (q, path) = temp_queue("garbage");
    q.push(&sample(1));
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, b"not json\n"))
        .unwrap();
    q.push(&sample(2));
    let sent = RefCell::new(vec![]);
    q.flush(|s| {
        sent.borrow_mut().push(s.date_time);
        Ok(())
    })
    .unwrap();
    assert_eq!(*sent.borrow(), vec![1, 2]);
}

#[test]
fn status_classification() {
    assert!(matches!(SendError::from_status(422, String::new()), SendError::Permanent(_)));
    assert!(matches!(SendError::from_status(413, String::new()), SendError::Permanent(_)));
    for s in [401, 403, 429, 500, 503] {
        assert!(matches!(SendError::from_status(s, String::new()), SendError::Retry(_)), "{s}");
    }
}
