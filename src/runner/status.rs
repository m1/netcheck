use chrono::{DateTime, TimeDelta, Utc};

/// Status is a struct that holds the status of a run.
#[derive(Default, Debug)]
pub struct Status {
    pub threshold: u8,
    pub unavailable_count: i32,
    pub unavailable_started: DateTime<Utc>,
    pub unavailable_counted: u8,
    pub is_unavailable: bool,
    pub available_counted: u8,
    pub unavailable_events: Vec<Event>,
}

impl Status {
    /// Create a new Status.
    ///
    /// # Arguments
    ///
    /// * `threshold`: A u8 that holds the threshold to determine if a target is
    /// unavailable or available.
    ///
    /// returns: Status
    pub fn new(threshold: u8) -> Status {
        Status {
            threshold,
            ..Default::default()
        }
    }

    #[tracing::instrument(level = "trace")]
    pub fn handle_available(&mut self) -> Event {
        self.available_counted += 1;
        if self.is_unavailable {
            if self.available_counted >= self.threshold {
                self.is_unavailable = false;
                self.unavailable_counted = 0;
                let evt = Event::UnavailableToAvailable(chrono::Utc::now() - self.unavailable_started);
                self.unavailable_events.push(evt);

                return evt;
            }
        }

        self.unavailable_counted = 0;

        Event::NoChange
    }

    #[tracing::instrument(level = "trace")]
    pub fn handle_unavailable(&mut self) -> Event {
        self.unavailable_counted += 1;
        if !self.is_unavailable {
            if self.unavailable_counted >= self.threshold {
                self.is_unavailable = true;
                self.available_counted = 0;

                self.unavailable_started = chrono::Utc::now();
                self.unavailable_count += 1;

                return Event::AvailableToUnavailable;
            }
        }

        self.available_counted = 0;

        Event::NoChange
    }
}

/// Event is an enum that holds the event of a single run.
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Event {
    AvailableToUnavailable,
    UnavailableToAvailable(TimeDelta),
    NoChange,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_status_new() {
        let status = Status::new(3);
        assert_eq!(status.threshold, 3);
        assert_eq!(status.unavailable_count, 0);
        assert_eq!(status.unavailable_counted, 0);
        assert_eq!(status.is_unavailable, false);
        assert_eq!(status.available_counted, 0);
        assert_eq!(status.unavailable_events, vec![]);
    }

    #[test]
    fn test_status_handle_available() {
        let mut status = Status::new(3);
        status.is_unavailable = true;
        status.unavailable_started = chrono::Utc::now();
        assert_eq!(status.handle_available(), Event::NoChange);
        assert_eq!(status.handle_available(), Event::NoChange);
        match status.handle_available() {
            Event::UnavailableToAvailable(duration) => assert!(duration.num_nanoseconds().expect("should unwrap") > 0),
            _ => panic!("Expected UnavailableToAvailable"),
        }
    }

    #[test]
    fn test_status_handle_unavailable() {
        let mut status = Status::new(3);
        assert_eq!(status.handle_available(), Event::NoChange);
        assert_eq!(status.handle_available(), Event::NoChange);
        assert_eq!(status.handle_available(), Event::NoChange);
        assert_eq!(status.handle_unavailable(), Event::NoChange);
        assert_eq!(status.handle_unavailable(), Event::NoChange);
        assert_eq!(status.handle_unavailable(), Event::AvailableToUnavailable);
    }
}
