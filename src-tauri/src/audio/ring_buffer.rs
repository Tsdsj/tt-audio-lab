use std::collections::VecDeque;

// 简易环形队列：满时覆盖最老数据，避免无限增长。
#[derive(Debug, Default)]
pub struct RingBuffer<T> {
    data: VecDeque<T>,
    max_len: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(max_len: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(max_len),
            max_len,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.data.len() == self.max_len {
            let _ = self.data.pop_front();
        }
        self.data.push_back(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.data.pop_front()
    }
}
