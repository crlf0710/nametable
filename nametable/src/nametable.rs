use std::hash::{Hash, Hasher, SipHasher};

pub fn name_hash(str_: &str) -> u64 {
    let mut hasher = SipHasher::new();
    str_.hash(&mut hasher);
    hasher.finish()
}

pub trait NameTableIdx {
    fn to_index(&self) -> usize;
}

impl NameTableIdx for usize {
    fn to_index(&self) -> usize {
        *self
    }
}

pub trait NameTable {
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>>;

    fn initial_local(&self) -> usize;
    fn len_local(&self) -> usize;
    fn at_local<'a>(&'a self, idx: usize) -> &'a str;
    fn find_local(&self, name: &str) -> Option<usize> {
        for i in 0..self.len_local() {
            if self.at_local(i) == name {
                return Some(i);
            }
        }
        return None;
    }

    fn len(&self) -> usize {
        return self.len_local() + self.parent().map_or(0usize, |parent_table| parent_table.len());
    }

    fn at<'a>(&'a self, idx: usize) -> &'a str {
        let initial = self.initial_local();
        if idx >= initial {
            return self.at_local(idx - initial);
        } else if let Some(parent_table) = self.parent().as_ref() {
            &parent_table.at(idx)
        } else {
            panic!("access out of bound");
        }
    }

    fn find(&self, name: &str) -> Option<usize> {
        let initial = self.initial_local();
        self.find_local(name).map(|idx| idx + initial).or_else(|| {
            if let Some(parent_table) = self.parent().as_ref() {
                return parent_table.find(name);
            }
            return None;
        })
    }
}

pub struct StaticNameTable<'a> {
    initial_idx: usize,
    names: &'static str,
    name_offsets: &'static [usize],
    parent: Option<Box<NameTable + 'a>>,
}

impl<'x> NameTable for StaticNameTable<'x> {
    fn initial_local(&self) -> usize {
        self.initial_idx
    }
    fn len_local(&self) -> usize {
        self.name_offsets.len() - 1
    }
    fn at_local<'a>(&'a self, idx: usize) -> &'a str {
        &self.names[self.name_offsets[idx]..self.name_offsets[idx + 1]]
    }
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>> {
        self.parent.as_ref()
    }
}

impl<'x> StaticNameTable<'x> {
    pub fn new(names_: &'static str, name_offsets_: &'static [usize]) -> Self {
        return StaticNameTable {
            initial_idx: 0usize,
            names: names_,
            name_offsets: name_offsets_,
            parent: None,
        };
    }

    pub fn new_upon<ParentTableType: 'x + NameTable>(names_: &'static str,
                                                     name_offsets_: &'static [usize],
                                                     parent: ParentTableType)
                                                     -> Self {
        return StaticNameTable {
            initial_idx: parent.initial_local() + parent.len_local(),
            names: names_,
            name_offsets: name_offsets_,
            parent: Some(Box::new(parent)),
        };
    }

    pub fn index<T: NameTableIdx>(&'x self, idx: T) -> &'x str {
        &self.at(idx.to_index())
    }
}

pub struct DynamicNameTable<'a> {
    initial_idx: usize,
    names: Box<Vec<String>>,
    parent: Option<Box<NameTable + 'a>>,
}

impl<'x> NameTable for DynamicNameTable<'x> {
    fn initial_local(&self) -> usize {
        self.initial_idx
    }
    fn len_local(&self) -> usize {
        self.names.len()
    }
    fn at_local<'a>(&'a self, idx: usize) -> &'a str {
        self.names[idx].as_str()
    }
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>> {
        self.parent.as_ref()
    }
}

impl<'x> DynamicNameTable<'x> {
    pub fn new() -> Self {
        return DynamicNameTable {
            initial_idx: 0usize,
            names: Box::new(Vec::new()),
            parent: None,
        };
    }

    pub fn new_upon<ParentTableType: 'x + NameTable>(parent: ParentTableType) -> Self {
        return DynamicNameTable {
            initial_idx: parent.initial_local() + parent.len_local(),
            names: Box::new(Vec::new()),
            parent: Some(Box::new(parent)),
        };
    }

    pub fn intern(&mut self, name: &str) -> usize {
        self.find(name)
            .or_else(|| {
                Some({
                    self.names.as_mut().push(name.to_owned());
                    self.initial_idx + self.names.len() - 1
                })
            })
            .unwrap()
    }

    pub fn index<T: NameTableIdx>(&'x self, idx: T) -> &'x str {
        &self.at(idx.to_index())
    }
}


pub struct StaticHashedNameTable<'a> {
    initial_idx: usize,
    names: &'static str,
    name_offsets: &'static [usize],
    hash_idxes: &'static [(u64, usize)],
    parent: Option<Box<NameTable + 'a>>,
}

impl<'x> StaticHashedNameTable<'x> {
    pub fn new(names_: &'static str,
               name_offsets_: &'static [usize],
               hash_idxes_: &'static [(u64, usize)])
               -> Self {

        return StaticHashedNameTable {
            initial_idx: 0usize,
            names: names_,
            name_offsets: name_offsets_,
            hash_idxes: hash_idxes_,
            parent: None,
        };
    }

    pub fn new_upon<ParentTableType: 'x + NameTable>(names_: &'static str,
                                                     name_offsets_: &'static [usize],
                                                     hash_idxes_: &'static [(u64, usize)],
                                                     parent: ParentTableType)
                                                     -> Self {

        return StaticHashedNameTable {
            initial_idx: parent.initial_local() + parent.len_local(),
            names: names_,
            name_offsets: name_offsets_,
            hash_idxes: hash_idxes_,
            parent: Some(Box::new(parent)),
        };
    }

    pub fn index<T: NameTableIdx>(&'x self, idx: T) -> &'x str {
        &self.at(idx.to_index())
    }

    fn hash_enabled(&self) -> bool {
        self.hash_idxes.len() != 0
    }

    fn find_local_hashing(&self, name: &str) -> Option<usize> {
        let target = name_hash(name);
        let result = self.hash_idxes.binary_search_by(|&(a, _)| a.cmp(&target));
        match result {
            Ok(val) => {
                if self.at_local(self.hash_idxes[val].1) == name {
                    Some(self.hash_idxes[val].1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn find_local_fallback(&self, name: &str) -> Option<usize> {
        for i in 0..self.len_local() {
            if self.at_local(i) == name {
                return Some(i);
            }
        }
        return None;
    }
}

impl<'x> NameTable for StaticHashedNameTable<'x> {
    fn initial_local(&self) -> usize {
        self.initial_idx
    }
    fn len_local(&self) -> usize {
        self.name_offsets.len() - 1
    }
    fn at_local<'a>(&'a self, idx: usize) -> &'a str {
        &self.names[self.name_offsets[idx]..self.name_offsets[idx + 1]]
    }
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>> {
        self.parent.as_ref()
    }

    fn find_local(&self, name: &str) -> Option<usize> {
        if self.hash_enabled() {
            self.find_local_hashing(name)
        } else {
            self.find_local_fallback(name)
        }
    }
}
