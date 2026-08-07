use std::collections::{VecDeque};

pub struct Reporter {
    queue:VecDeque<String>
}

impl Reporter {
    pub fn push(&mut self, data:&str){
        self.queue.push_back(data.to_string()); 
    }

    pub fn publish(&mut self) {
        while !self.queue.is_empty() {
            println!("{}", self.queue.back().unwrap()) ;
            self.queue.pop_back();
        }
    }

    pub fn new()->Self {
        Reporter { queue: VecDeque::new() }
    }
}

impl Default for  Reporter{
    fn default() -> Self {
        Self::new()
    } 
}
