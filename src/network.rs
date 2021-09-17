// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cita_cloud_proto::network::network_service_client::NetworkServiceClient;
use cita_cloud_proto::network::RegisterInfo;
use log::warn;
use status_code::StatusCode;
use tonic::transport::Channel;
use tonic::Request;

pub async fn register_network_msg_handler(
    mut client: NetworkServiceClient<Channel>,
    register_info: RegisterInfo,
) -> StatusCode {
    let request = Request::new(register_info);

    match client.register_network_msg_handler(request).await {
        Ok(res) => StatusCode::from(res.into_inner().code),
        Err(status) => {
            warn!("register_network_msg_handler error: {}", status.to_string());
            StatusCode::NetworkServerNotReady
        }
    }
}
