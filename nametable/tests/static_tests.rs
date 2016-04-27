use nametable::*;

#[derive(Clone,Copy)]
enum NameEnum1 {
    FIRST = 0,
    SECOND = 1,
    THIRD = 2,
}

impl NameTableIdx for NameEnum1 {
    fn to_index(&self) -> usize {
        *self as usize
    }
}

#[repr(usize)]
#[derive(Clone,Copy)]
enum NameEnum2 {
    FOURTH = 3,
    FIFTH = 4,
    SIXTH = 5,
}

impl NameTableIdx for NameEnum2 {
    fn to_index(&self) -> usize {
        *self as usize
    }
}

static NAME_DATA_1: &'static str = "FIRSTSECONDTHIRD";
static INDEX_DATA_1: &'static [usize] = &[0, 5, 11, 16];

static NAME_DATA_2: &'static str = "FOURTHFIFTHSIXTHSEVENTH";
static INDEX_DATA_2: &'static [usize] = &[0, 6, 11, 16, 23];


#[test]
fn test1() {
    let tbl = StaticNameTable::new(NAME_DATA_1, INDEX_DATA_1);
    assert!(tbl.len() == 3);
    assert!(tbl.parent().is_none());

    assert!(tbl.initial_local() == 0);
    assert!(tbl.len_local() == 3);
    assert!(tbl.at_local(0) == "FIRST");
    assert!(tbl.at_local(1) == "SECOND");
    assert!(tbl.at_local(2) == "THIRD");

    assert!(tbl.find_local("FIRST").unwrap() == 0);
    assert!(tbl.find_local("SECOND").unwrap() == 1);
    assert!(tbl.find_local("THIRD").unwrap() == 2);
    assert!(tbl.find_local("UNEXIST").is_none());

    assert!(tbl.at(0) == "FIRST");
    assert!(tbl.at(1) == "SECOND");
    assert!(tbl.at(2) == "THIRD");

    assert!(tbl.find("FIRST").unwrap() == 0);
    assert!(tbl.find("SECOND").unwrap() == 1);
    assert!(tbl.find("THIRD").unwrap() == 2);
    assert!(tbl.find("UNEXIST").is_none());

    assert!(&tbl[0] == "FIRST");
    assert!(&tbl[NameEnum1::FIRST] == "FIRST");

    assert!(&tbl[0] == "FIRST");
    assert!(&tbl[NameEnum1::FIRST] == "FIRST");
}


#[test]
fn test2() {
    let tbl = StaticNameTable::new_upon(NAME_DATA_2,
                                        INDEX_DATA_2,
                                        StaticNameTable::new(NAME_DATA_1, INDEX_DATA_1));

    assert!(tbl.len() == 7);
    assert!(tbl.parent().is_some());

    assert!(tbl.initial_local() == 3);
    assert!(tbl.len_local() == 4);
    assert!(tbl.at_local(0) == "FOURTH");
    assert!(tbl.at_local(1) == "FIFTH");
    assert!(tbl.at_local(2) == "SIXTH");
    assert!(tbl.at_local(3) == "SEVENTH");

    assert!(tbl.find_local("FOURTH").unwrap() == 0);
    assert!(tbl.find_local("FIFTH").unwrap() == 1);
    assert!(tbl.find_local("FIRST").is_none());
    assert!(tbl.find_local("UNEXIST").is_none());

    assert!(tbl.at(0) == "FIRST");
    assert!(tbl.at(1) == "SECOND");
    assert!(tbl.at(2) == "THIRD");
    assert!(tbl.at(3) == "FOURTH");
    assert!(tbl.at(6) == "SEVENTH");

    assert!(tbl.find("FIRST").unwrap() == 0);
    assert!(tbl.find("THIRD").unwrap() == 2);
    assert!(tbl.find("FOURTH").unwrap() == 3);
    assert!(tbl.find("SEVENTH").unwrap() == 6);
    assert!(tbl.find("UNEXIST").is_none());

    assert!(&tbl[0] == "FIRST");
    assert!(&tbl[NameEnum1::FIRST] == "FIRST");
    assert!(&tbl[NameEnum2::FOURTH] == "FOURTH");
}
