//! The channel exists in M1 even though both halves are in one app, because M3
//! swaps a postMessage transport underneath it. These tests pin the semantics
//! that swap has to preserve.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus_storybook_core::{Channel, Event, InProcessChannel};

fn recorder() -> (Rc<RefCell<Vec<Event>>>, Rc<dyn Fn(&Event)>) {
    let log: Rc<RefCell<Vec<Event>>> = Rc::default();
    let sink = Rc::clone(&log);
    (log, Rc::new(move |e: &Event| sink.borrow_mut().push(e.clone())))
}

#[test]
fn every_listener_sees_every_event() {
    let ch = InProcessChannel::new();
    let (a, fa) = recorder();
    let (b, fb) = recorder();
    let _sa = ch.subscribe(fa);
    let _sb = ch.subscribe(fb);

    ch.emit(Event::SetCurrentStory { id: "x".into() });
    assert_eq!(a.borrow().len(), 1);
    assert_eq!(b.borrow().len(), 1);
}

#[test]
fn dropping_the_subscription_unsubscribes() {
    let ch = InProcessChannel::new();
    let (log, f) = recorder();
    let sub = ch.subscribe(f);
    assert_eq!(ch.listener_count(), 1);

    ch.emit(Event::SetCurrentStory { id: "x".into() });
    drop(sub);
    assert_eq!(ch.listener_count(), 0);
    ch.emit(Event::SetCurrentStory { id: "y".into() });

    assert_eq!(log.borrow().len(), 1, "no delivery after unsubscribe");
}

#[test]
fn a_listener_may_emit_while_being_dispatched() {
    // The preview answers SetCurrentStory with StoryRendered from inside the
    // dispatch. A naive RefCell-held listener list would panic here.
    let ch = InProcessChannel::new();
    let (log, f) = recorder();
    let _sub = ch.subscribe(f);

    let echo = ch.clone();
    let _sub2 = ch.subscribe(Rc::new(move |e: &Event| {
        if let Event::SetCurrentStory { id } = e {
            echo.emit(Event::StoryRendered { id: id.clone() });
        }
    }));

    ch.emit(Event::SetCurrentStory { id: "x".into() });

    let seen = log.borrow();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0], Event::SetCurrentStory { id: "x".into() });
    assert_eq!(seen[1], Event::StoryRendered { id: "x".into() });
}

#[test]
fn subscribing_during_dispatch_does_not_disturb_the_current_broadcast() {
    let ch = InProcessChannel::new();
    let inner = ch.clone();
    let (log, f) = recorder();
    let _sub = ch.subscribe(Rc::new(move |_| {
        let _leaked = inner.subscribe(Rc::new(|_| {}));
        std::mem::forget(_leaked);
    }));
    let _sub2 = ch.subscribe(f);

    ch.emit(Event::ResetArgs { id: "x".into() });
    assert_eq!(log.borrow().len(), 1);
}

#[test]
fn clones_share_one_listener_list() {
    let ch = InProcessChannel::new();
    let (log, f) = recorder();
    let _sub = ch.subscribe(f);

    ch.clone().emit(Event::StoryMissing { id: "gone".into() });
    assert_eq!(log.borrow().len(), 1);
}
