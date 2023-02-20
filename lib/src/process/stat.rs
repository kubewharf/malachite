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

pub enum ProcStatFileIndex {
    Pid,
    CommandLine,
    TaskState,
    ParentPid,
    ParentGroupId,
    SessionId,
    TaskFlags,
    /// UserTime (jiffies)
    UserTime,
    /// SysTime (jiffies)
    SysTime,
    StartTime,
    /// VSize(page)
    VSize,
    /// Rss(page)
    Rss,
}

impl ProcStatFileIndex {
    pub fn as_index(&self) -> usize {
        match *self {
            ProcStatFileIndex::Pid => 0_usize,
            ProcStatFileIndex::CommandLine => 1_usize,
            ProcStatFileIndex::TaskState => 2_usize,
            ProcStatFileIndex::ParentPid => 3_usize,
            ProcStatFileIndex::ParentGroupId => 4_usize,
            ProcStatFileIndex::SessionId => 5_usize,
            ProcStatFileIndex::TaskFlags => 8_usize,
            ProcStatFileIndex::UserTime => 13_usize,
            ProcStatFileIndex::SysTime => 14_usize,
            ProcStatFileIndex::StartTime => 21_usize,
            ProcStatFileIndex::VSize => 22_usize,
            ProcStatFileIndex::Rss => 23_usize,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat() {
        let index = ProcStatFileIndex::Pid;
        assert_eq!(index.as_index(), 0_usize);

        let index = ProcStatFileIndex::CommandLine;
        assert_eq!(index.as_index(), 1_usize);

        let index = ProcStatFileIndex::TaskState;
        assert_eq!(index.as_index(), 2_usize);

        let index = ProcStatFileIndex::ParentPid;
        assert_eq!(index.as_index(), 3_usize);

        let index = ProcStatFileIndex::ParentGroupId;
        assert_eq!(index.as_index(), 4_usize);

        let index = ProcStatFileIndex::SessionId;
        assert_eq!(index.as_index(), 5_usize);

        let index = ProcStatFileIndex::TaskFlags;
        assert_eq!(index.as_index(), 8_usize);

        let index = ProcStatFileIndex::UserTime;
        assert_eq!(index.as_index(), 13_usize);

        let index = ProcStatFileIndex::SysTime;
        assert_eq!(index.as_index(), 14_usize);

        let index = ProcStatFileIndex::StartTime;
        assert_eq!(index.as_index(), 21_usize);

        let index = ProcStatFileIndex::VSize;
        assert_eq!(index.as_index(), 22_usize);

        let index = ProcStatFileIndex::Rss;
        assert_eq!(index.as_index(), 23_usize);
    }
}
