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

use rocket::serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

pub(crate) struct Resp<T> {
    pub status: i32,
    pub data: Result<T, &'static str>,
}

impl<T> Resp<T> {
    pub(crate) fn new(t: T) -> Resp<T> {
        Resp::<T> {
            status: 0,
            data: Ok(t),
        }
    }
}

impl<T> Serialize for Resp<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Resp", 2)?;
        state.serialize_field("status", &self.status)?;
        match &self.data {
            Ok(d) => {
                state.serialize_field("data", d)?;
            }
            Err(i) => {
                state.serialize_field("message", i)?;
            }
        }
        state.end()
    }
}
