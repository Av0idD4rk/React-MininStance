use crate::error::DeployError;
use bollard::Docker;
use bollard::auth::DockerCredentials;
use bollard::models::{ContainerCreateBody, HostConfig, PortBinding};
use bollard::query_parameters::{BuildImageOptions, RestartContainerOptions};
use bollard::query_parameters::{
    CreateContainerOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use http_body_util::{Either, Full, StreamBody};
use hyper::body::Frame;
use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use tar::Builder as TarBuilder;

type BodyType = Either<
    Full<Bytes>,
    StreamBody<Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, io::Error>> + Send>>>,
>;

pub struct DockerClient {
    inner: Docker, // keep this private
}

impl DockerClient {
    pub fn new() -> Self {
        let docker = Docker::connect_with_local_defaults().unwrap();
        Self { inner: docker }
    }

    pub async fn build_image(&self, task_name: &str, tag: &str) -> Result<(), DeployError> {
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

        let mut build_stream = self.inner.build_image(
            options,
            None::<HashMap<String, DockerCredentials>>,
            Some(body),
        );

        while let Some(chunk) = build_stream.try_next().await? {
            if let Some(err_msg) = chunk.error {
                return Err(DeployError::Build(err_msg));
            }
        }
        Ok(())
    }
    pub async fn create_container(
        &self,
        opts: CreateContainerOptions,
        body: ContainerCreateBody,
    ) -> Result<String, DeployError> {
        let info = self.inner.create_container(Some(opts), body).await?;
        Ok(info.id)
    }
    pub async fn start_container(
        &self,
        container_id: &str,
        options: Option<StartContainerOptions>,
    ) -> Result<(), DeployError> {
        self.inner.start_container(container_id, options).await?;
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<(), DeployError> {
        let _ = self.inner.stop_container(container_id, None::<StopContainerOptions>).await;
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<(), DeployError> {
        let _ = self.inner.remove_container(
            container_id,
            Some(RemoveContainerOptions { force: true, ..Default::default() }),
        ).await;
        Ok(())
    }

    pub async fn restart_container(&self, container_id: &str) -> Result<(), DeployError> {
        self.inner.restart_container(container_id, None::<RestartContainerOptions>).await?;
        Ok(())
    }
}
