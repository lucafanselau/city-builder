//! This event system aims the be easily usable with the ecs system in place

use std::cell::RefMut;

pub trait Event = 'static;

pub struct EventBuffer<T: Event> {
    buffer: Vec<T>,
}

enum EventState {
    Alpha,
    Beta,
}

pub struct Events<T: Event> {
    state: EventState,
    alpha: EventBuffer<T>,
    beta: EventBuffer<T>,
}

impl<T: Event> Default for Events<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Event> Events<T> {
    pub fn new() -> Self {
        Self {
            state: EventState::Alpha,
            alpha: EventBuffer { buffer: Vec::new() },
            beta: EventBuffer { buffer: Vec::new() },
        }
    }

    fn get_buffer_mut(&mut self) -> &mut EventBuffer<T> {
        match self.state {
            EventState::Alpha => &mut self.alpha,
            EventState::Beta => &mut self.beta,
        }
    }

    pub fn send(&mut self, instance: T) {
        let buffer = self.get_buffer_mut();
        buffer.buffer.push(instance);
    }

    // NOTE(luca): If we feel like we actually need an event reader
    // pub fn get_reader(&self) -> EventReader<T> {
    //     EventReader {
    //         last_count: 0,
    //         _marker: PhantomData,
    //     }
    // }

    /// For now we will enable only last frame iteration
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        match self.state {
            EventState::Alpha => self.beta.buffer.iter(),
            EventState::Beta => self.alpha.buffer.iter(),
        }
    }

    pub fn update(&mut self) {
        match self.state {
            EventState::Alpha => {
                self.state = EventState::Beta;
                self.beta = EventBuffer { buffer: Vec::new() }
            }
            EventState::Beta => {
                self.state = EventState::Alpha;
                self.alpha = EventBuffer { buffer: Vec::new() }
            }
        }
    }

    pub fn update_system(mut events: RefMut<Self>) {
        events.update()
    }
}

#[cfg(test)]
mod tests {
    use super::Events;

    #[derive(Debug, PartialEq)]
    struct NumberEvent(i32);

    #[test]
    fn simple_events() {
        let mut events = Events::new();

        events.send(NumberEvent(2));

        events.update();
        {
            assert_eq!(events.iter().next(), Some(&NumberEvent(2)));
            events.send(NumberEvent(3));
            events.send(NumberEvent(4));
        }
        events.update();
        {
            let mut iter = events.iter();
            assert_eq!(iter.next(), Some(&NumberEvent(3)));
            assert_eq!(iter.next(), Some(&NumberEvent(4)));
            assert_eq!(iter.next(), None);
        }
    }
}
