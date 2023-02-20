/*
Copyright 2023 The Malachite Authors.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

use std::fmt;

///         `<https://man7.org/linux/man-pages/man1/ps.1.html#PROCESS_STATE_CODES>`
///
///         D    uninterruptible sleep (usually IO)
///         I    Idle kernel thread
///         R    running or runnable (on run queue)
///         S    interruptible sleep (waiting for an event to complete)
///         T    stopped by job control signal
///         t    stopped by debugger during the tracing
///         W    paging (not valid since the 2.6.xx kernel)
///         X    dead (should never be seen)
///         Z    defunct ("zombie") process, terminated but not reaped by its parent
///
#[derive(Clone, Copy, Debug)]
pub enum ProcessStatus {
    /// uninterruptible sleep (usually IO)
    UninterruptibleSleep,
    /// idle kernel thread
    Idle,
    /// running or runnable (on run queue)
    Run,
    /// interruptable sleep
    Sleep,
    /// stopped by job control signal
    Stop,
    /// stopped by debugger during the tracing
    Tracing,
    /// paging (not valid since the 2.6.xx kernel)
    Paging,
    /// dead (should never be seen)
    Dead,
    /// zombie process
    Zombie,
    /// unknown
    Unknown(u32),
}

impl ProcessStatus {
    pub fn as_str(&self) -> &str {
        match *self {
            ProcessStatus::UninterruptibleSleep => "UninterruptibleSleep",
            ProcessStatus::Idle => "Idle",
            ProcessStatus::Run => "Run",
            ProcessStatus::Sleep => "Sleep",
            ProcessStatus::Stop => "Stop",
            ProcessStatus::Tracing => "Tracing",
            ProcessStatus::Paging => "Paging",
            ProcessStatus::Dead => "Dead",
            ProcessStatus::Zombie => "Zombie",
            ProcessStatus::Unknown(_) => "Unknown",
        }
    }
}

#[cfg(not(tarpaulin_include))]
impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<char> for ProcessStatus {
    fn from(status: char) -> ProcessStatus {
        match status {
            'D' => ProcessStatus::UninterruptibleSleep,
            'I' => ProcessStatus::Idle,
            'R' => ProcessStatus::Run,
            'S' => ProcessStatus::Sleep,
            'T' => ProcessStatus::Stop,
            't' => ProcessStatus::Tracing,
            'W' => ProcessStatus::Paging,
            'X' => ProcessStatus::Dead,
            'Z' => ProcessStatus::Zombie,
            x => ProcessStatus::Unknown(x as u32),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_process_status() {
        let process = ProcessStatus::UninterruptibleSleep;
        assert_eq!(process.as_str(), "UninterruptibleSleep");
        assert_eq!(process.as_str(), ProcessStatus::from('D').as_str());

        let process = ProcessStatus::Idle;
        assert_eq!(process.as_str(), "Idle");
        assert_eq!(process.as_str(), ProcessStatus::from('I').as_str());

        let process = ProcessStatus::Run;
        assert_eq!(process.as_str(), "Run");
        assert_eq!(process.as_str(), ProcessStatus::from('R').as_str());

        let process = ProcessStatus::Sleep;
        assert_eq!(process.as_str(), "Sleep");
        assert_eq!(process.as_str(), ProcessStatus::from('S').as_str());

        let process = ProcessStatus::Stop;
        assert_eq!(process.as_str(), "Stop");
        assert_eq!(process.as_str(), ProcessStatus::from('T').as_str());

        let process = ProcessStatus::Tracing;
        assert_eq!(process.as_str(), "Tracing");
        assert_eq!(process.as_str(), ProcessStatus::from('t').as_str());

        let process = ProcessStatus::Paging;
        assert_eq!(process.as_str(), "Paging");
        assert_eq!(process.as_str(), ProcessStatus::from('W').as_str());

        let process = ProcessStatus::Dead;
        assert_eq!(process.as_str(), "Dead");
        assert_eq!(process.as_str(), ProcessStatus::from('X').as_str());

        let process = ProcessStatus::Zombie;
        assert_eq!(process.as_str(), "Zombie");
        assert_eq!(process.as_str(), ProcessStatus::from('Z').as_str());

        let process = ProcessStatus::Unknown(0);
        assert_eq!(process.as_str(), "Unknown");
        assert_eq!(process.as_str(), ProcessStatus::from('0').as_str());
    }
}
