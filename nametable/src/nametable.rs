use std::ops::Index;

pub trait NameTable {
    fn initial_local(&self) -> usize;
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>>;

    fn size_local(&self) -> usize;
    fn at_local<'a>(&'a self, idx: usize) -> &'a str;
    fn find_local(&self, name: &str) -> Option<usize> {
        for i in 0..self.size_local() {
            if self.at_local(i) == name {
                return Some(i);
            }
        }
        return None
    }

    fn size(&self) -> usize {
        return self.size_local() +
            self.parent().map_or(0usize, |parent_table| parent_table.size());
    }

    fn at<'a>(&'a self, idx: usize) -> &'a str {
        let initial = self.initial_local();
        if idx >= initial {
            return self.at_local(idx - initial)
        }
        else if let Some(parent_table) = self.parent().as_ref() {
            &parent_table.at(idx)
        }
        else {
            panic!("access out of bound");
        }
    }

    fn find(&self, name: &str) -> Option<usize> {
        let initial = self.initial_local();
        self.find_local(name).map(|idx| idx + initial).or_else(|| {
            if let Some(parent_table) = self.parent().as_ref() {
                return parent_table.find(name)
            }
            return None
        })
    }
}

impl Index<usize> for NameTable {
    type Output = str;
    fn index(&self, idx: usize) -> &str {
        self.at(idx)
    }
}

pub struct StaticNameTable<'a> {
    initial_idx: usize,
    names : &'static str,
    name_offsets : &'static [usize],
    parent : Option<Box<NameTable + 'a>>,
}

impl<'x> NameTable for StaticNameTable<'x> {
    fn initial_local(&self) -> usize { self.initial_idx }
    fn size_local(&self) -> usize { self.name_offsets.len() - 1}
    fn at_local<'a>(&'a self, idx: usize) -> &'a str { &self.names[self.name_offsets[idx]..self.name_offsets[idx + 1]] }
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>> { self.parent.as_ref()}
}

pub struct DynamicNameTable<'a> {
    initial_idx: usize,
    names : Box<Vec<String>>,
    parent : Option<Box<NameTable + 'a>>,
}

impl<'x> NameTable for DynamicNameTable<'x> {
    fn initial_local(&self) -> usize { self.initial_idx }
    fn size_local(&self) -> usize { self.names.len() }
    fn at_local<'a>(&'a self, idx: usize) -> &'a str { self.names[idx].as_str() }
    fn parent<'a>(&'a self) -> Option<&Box<NameTable + 'a>> { self.parent.as_ref()}
}

impl<'x> DynamicNameTable<'x> {
    fn intern(&mut self, name: &str) -> usize {
        self.find(name).or_else(|| Some({
            self.names.as_mut().push(name.to_owned());
            self.initial_idx + self.names.len() - 1
        })).unwrap()
    }
}

