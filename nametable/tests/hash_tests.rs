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
static HASH_DATA_1: &'static [(u64, usize)] = &[];

static NAME_DATA_2: &'static str = "FOURTHFIFTHSIXTHSEVENTH";
static INDEX_DATA_2: &'static [usize] = &[0, 6, 11, 16, 23];
static mut HASH_DATA_HOLDER_2: [(u64, usize); 4] = [(0, 0), (0, 1), (0, 2), (0, 3)];

use std::sync::{Once, ONCE_INIT};

static START: Once = ONCE_INIT;

#[test]
fn test1() {
    START.call_once(|| {
        unsafe {
            HASH_DATA_HOLDER_2[0].0 = name_hash("FOURTH");
            HASH_DATA_HOLDER_2[1].0 = name_hash("FIFTH");
            HASH_DATA_HOLDER_2[2].0 = name_hash("SIXTH");
            HASH_DATA_HOLDER_2[3].0 = name_hash("SEVENTH");
            HASH_DATA_HOLDER_2.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
        }
    });
    let hash_data_2: &'static [(u64, usize)] = unsafe { &HASH_DATA_HOLDER_2 };

    let tbl = StaticHashedNameTable::new_upon(NAME_DATA_2,
                                              INDEX_DATA_2,
                                              hash_data_2,
                                              StaticHashedNameTable::new(NAME_DATA_1,
                                                                         INDEX_DATA_1,
                                                                         HASH_DATA_1));

    assert!(hash_data_2.len() == 4);
    let fourth_hash = name_hash("FOURTH");
    let result = hash_data_2.binary_search_by(|&(a, _)| a.cmp(&fourth_hash));
    assert!(result.is_ok());
    if let Ok(val) = result {
        assert!(tbl.at_local(hash_data_2[val].1) == "FOURTH");
    }


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

    assert!(tbl.index(0) == "FIRST");
    assert!(tbl.index(NameEnum1::FIRST) == "FIRST");
    assert!(tbl.index(NameEnum2::FOURTH) == "FOURTH");



}
