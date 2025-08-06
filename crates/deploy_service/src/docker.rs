use crate::error::DeployError;
use bollard::Docker;
use bollard::query_parameters::{CreateContainerOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions};
use bollard::query_parameters::BuildImageOptions;
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use futures_util::{TryStreamExt, Stream};
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use bollard::auth::DockerCredentials;
use bytes::Bytes;
use hyper::body::{Frame};
use http_body_util::{Either, Full, StreamBody};
use tar::Builder as TarBuilder;

type BodyType = Either<
    Full<Bytes>,
    StreamBody<Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, io::Error>> + Send>>>,
>;

pub struct DockerClient {
    inner: Docker,
}

impl DockerClient {
    pub fn new() -> Self {
        let docker = Docker::connect_with_local_defaults().unwrap();
        Self { inner: docker }
    }

    pub async fn build_image(
        &self,
        task_name: &str,
        tag: &str,
    ) -> Result<(), DeployError> {
        let options = BuildImageOptions {
            dockerfile: "Dockerfile".to_string(),
            t: Some(tag.to_string()),
            rm: true,
            ..Default::default()
        };

        let mut tar_buf = Vec::new();
        {
            let mut tar = TarBuilder::new(&mut tar_buf);
            tar.append_dir_all(".", format!("tasks/{}", task_name))?;
            tar.finish()?;
        }
        let full = Full::from(Bytes::from(tar_buf));
        let body: BodyType = Either::Left(full);

        let mut build_stream = self
            .inner
            .build_image(options, None::<HashMap<String, DockerCredentials>>, Some(body));

        while let Some(chunk) = build_stream.try_next().await? {
            if let Some(err_msg) = chunk.error {
                return Err(DeployError::Build(err_msg));
            }
        }
        Ok(())
    }
    pub async fn start_container(
        &self,
        tag: &str,
        host_port: u16,
        container_port: &str,
    ) -> Result<String, DeployError> {
        let mut bindings = HashMap::new();
        bindings.insert(
            format!("{}/tcp", container_port),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".into()),
                host_port: Some(host_port.to_string()),
            }]),
        );

        let opts = CreateContainerOptions {
            name: Some(tag.to_string()),
            platform: "".to_string(),
        };

        let create_body = ContainerCreateBody {
            image: Some(tag.to_string()),
            host_config: Some(HostConfig {
                port_bindings: Some(bindings),
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self.inner.create_container(Some(opts), create_body).await?;
        let id = container.id.clone();
        self.inner
            .start_container(&id, None::<StartContainerOptions>)
            .await?;
        Ok(id)
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<(), DeployError> {
        let _ = self.inner.stop_container(container_id, None::<StopContainerOptions>).await;
        let _ = self.inner.remove_container(container_id, None::<RemoveContainerOptions>).await;
        Ok(())
    }
}


