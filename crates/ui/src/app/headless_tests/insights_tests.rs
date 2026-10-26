//! The collection's Insights window, headless: Open Insights on the collection crate only,
//! bars that filter the collection, a dear record shown, and label loyalty with Follow.

use super::seller_tests::{Fakes, rig, with_token};
use super::*;
use crate::playlist::{ForSale, Origin, WaitKind};

/// A collection crate of these records (release, formats, year, style, label, price), one
/// entry each, and a crate of the user's own shown.
fn collection(name: &str, fakes: &Fakes) -> (Rig, CrateId) {
    let mut rig = rig(name, fakes, with_token);
    let c = rig.app.crates.create("Collection: digger").unwrap();
    rig.app.crates.set_collection(c);
    assert!(rig.app.crates.load(c));
    let records: [(u64, &str, u16, &str, &str, u64); 5] = [
        (1001, "Vinyl", 1991, "Hardcore", "Lowtide Tapes", 900),
        (1002, "Vinyl", 1995, "Jungle", "Lowtide Tapes", 22_250),
        (1003, "Vinyl, CD", 2021, "Electro", "Analogical Force", 1200),
        (1004, "CD", 1999, "Trance", "Somebody", 500),
        (1005, "File", 2020, "Trance", "Somebody", 100),
    ];
    let p = rig.app.crates.get_mut(c).unwrap();
    for (release, formats, year, style, label, cents) in records {
        p.add_waiting(
            "Artist",
            format!("Record {release}"),
            None,
            Some(Origin {
                release: Some(release),
                formats: formats.into(),
                year: Some(year),
                styles: style.into(),
                label: label.into(),
                artist: format!("Artist {release}"),
                album: format!("Record {release}"),
                for_sale: Some(ForSale {
                    count: 3,
                    lowest_cents: Some(cents),
                    currency: "EUR".into(),
                    fetched_at: 1_790_000_000,
                }),
                ..Default::default()
            }),
            WaitKind::Listed,
        );
    }
    rig.app.show_crate(PLAYLIST);
    rig.screen.x = 700.0;
    rig.frame(Vec::new());
    (rig, c)
}

fn open_insights(rig: &mut Rig) -> egui::FullOutput {
    let out = rig.frame(Vec::new());
    let pos = texts(&out)
        .into_iter()
        .find(|t| t.text == "Collection: digger")
        .unwrap_or_else(|| panic!("{:?}", text_list(&out)))
        .rect
        .center();
    rig.click_with(pos, PointerButton::Secondary);
    rig.click_text("Open Insights");
    rig.frame(Vec::new())
}

#[test]
fn open_insights_is_on_the_collection_only_and_counts_vinyl() {
    let fakes = Fakes::new();
    let (mut rig, _) = collection("insights-open", &fakes);
    let out = rig.frame(Vec::new());
    let pos = texts(&out)
        .into_iter()
        .find(|t| t.text == "Playlist")
        .unwrap()
        .rect
        .center();
    rig.click_with(pos, PointerButton::Secondary);
    let out = rig.frame(Vec::new());
    assert!(!shows(&out, "Open Insights"), "not on a crate of one's own");
    rig.key(Key::Escape, Modifiers::NONE);

    let out = open_insights(&mut rig);
    assert!(rig.app.insights_open);
    assert!(
        shows(&out, "INSIGHTS · 3 vinyl records"),
        "the CD and the File left out: {:?}",
        text_list(&out)
    );
    assert!(!shows(&out, "Trance"), "only on CD or File");
}

#[test]
fn a_bar_filters_the_collection_and_a_dear_record_is_shown() {
    let fakes = Fakes::new();
    let (mut rig, c) = collection("insights-filter", &fakes);
    open_insights(&mut rig);
    rig.click_text("Jungle");
    rig.frame(Vec::new());
    assert_eq!(rig.app.crates.shown_id(), c);
    let shown = rig.app.crates.shown();
    assert!(shown.picked(Facet::Style, "Jungle"));
    assert_eq!(shown.shown_rows().len(), 1, "the Jungle record only");
    assert!(rig.app.insights_open, "the window stays");

    // The 90s band: its years with records.
    rig.click_text("90s");
    let shown = rig.app.crates.shown();
    assert!(
        !shown.picked(Facet::Style, "Jungle"),
        "the other filters go"
    );
    assert!(shown.picked(Facet::Year, "1991") && shown.picked(Facet::Year, "1995"));
    assert_eq!(shown.shown_rows().len(), 2);

    // The dearest record: no filter, selected. The playlist row says the same; the window is
    // painted over it, so its entry is the last.
    let out = rig.frame(Vec::new());
    let pos = texts(&out)
        .into_iter()
        .rfind(|t| t.text == "Artist 1002 – Record 1002")
        .unwrap()
        .rect
        .center();
    rig.click(pos);
    let shown = rig.app.crates.shown();
    assert!(!shown.picks_filter(), "unfiltered");
    let id = shown
        .entries()
        .iter()
        .find(|e| e.title == "Record 1002")
        .unwrap()
        .id;
    assert_eq!(shown.selected_ids(), [id]);
}

#[test]
fn a_top_label_says_its_total_and_follows() {
    let fakes = Fakes::new();
    let (mut rig, _) = collection("insights-labels", &fakes);
    let out = open_insights(&mut rig);
    assert!(shows(&out, "2 of …"), "asking: {:?}", text_list(&out));
    // The intake's answer for Lowtide Tapes (its release 1001 names it, id 12345).
    rig.app
        .dig
        .as_mut()
        .unwrap()
        .label_totals
        .insert("Lowtide Tapes".into(), (12345, 61));
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "2 of 61 releases"), "{:?}", text_list(&out));
    rig.click_text("Follow");
    rig.until(
        |r| r.app.crates.find_label(12345).is_some(),
        "the label crate is made",
    );
    let out = rig.frame(Vec::new());
    assert!(shows(&out, "Following"), "{:?}", text_list(&out));
}

#[test]
fn the_collection_and_wantlist_are_never_renamed_or_deleted() {
    let fakes = Fakes::new();
    let (mut rig, c) = collection("collection-menu", &fakes);
    // The wantlist arrives with the first sync.
    rig.until(
        |r| r.app.crates.find("Wantlist: digger").is_some(),
        "the rig's wantlist",
    );
    let w = rig.app.crates.find("Wantlist: digger").unwrap();
    assert!(rig.app.crates.is_wantlist(w));
    for name in ["Collection: digger", "Wantlist: digger"] {
        let out = rig.frame(Vec::new());
        let pos = texts(&out)
            .into_iter()
            .find(|t| t.text == name)
            .unwrap_or_else(|| panic!("{name}: {:?}", text_list(&out)))
            .rect
            .center();
        rig.click_with(pos, PointerButton::Secondary);
        let out = rig.frame(Vec::new());
        assert!(
            !shows(&out, "Rename crate…") && !shows(&out, "Delete crate…"),
            "{name}: {:?}",
            text_list(&out)
        );
        rig.key(Key::Escape, Modifiers::NONE);
    }
    // Delete after a click on it in the sidebar: refused.
    rig.app.apply(Action::DeleteCrate(c), &rig.ctx.clone());
    assert!(rig.app.crates.info(c).is_some(), "still there");
    assert!(rig.app.confirm_delete.is_none(), "not even asked");
}
