// Copyright 2021 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use super::data::{Data, DataId};
use crate::types::{DataAddress, Map, MapAddress};

impl Data for Map {
    type Id = MapAddress;
    fn id(&self) -> &Self::Id {
        match self {
            Map::Seq(ref chunk) => chunk.address(),
            Map::Unseq(ref chunk) => chunk.address(),
        }
    }
}

impl DataId for MapAddress {
    fn to_data_address(&self) -> DataAddress {
        DataAddress::Map(*self)
    }
}
