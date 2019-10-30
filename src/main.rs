use rusoto_ecs::{
    Attribute, DescribeContainerInstancesRequest, Ecs, EcsClient, ListClustersRequest,
    ListContainerInstancesRequest,
};
use std::{error::Error, io::Write};
use structopt::StructOpt;
use tabwriter::TabWriter;

// https://aws.amazon.com/ec2/pricing/on-demand/
static INSTANCE_HOURLY: phf::Map<&'static str, f32> = phf::phf_map! {
    "m4.4xlarge" => 0.80,
    "c4.4xlarge" => 0.796,
    "r4.4xlarge" => 1.064,
    "c4.2xlarge" => 0.398,
    "t2.micro" => 0.0116,
    "m5.12xlarge" => 2.304,
    "r4.8xlarge" => 2.128
};

#[derive(StructOpt)]
/// Estimates what you may be spending on your ecs clusters
struct Ops {}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = Ops::from_args();
    let ecs = EcsClient::new(Default::default());
    let stdout = std::io::stdout();
    let mut writer = TabWriter::new(stdout);
    for cluster in ecs
        .list_clusters(ListClustersRequest {
            ..ListClustersRequest::default()
        })
        .sync()?
        .cluster_arns
        .unwrap_or_default()
    {
        let container_instances = ecs
            .list_container_instances(ListContainerInstancesRequest {
                cluster: Some(cluster.clone()),
                ..ListContainerInstancesRequest::default()
            })
            .sync()?
            .container_instance_arns
            .unwrap_or_default();
        let count = container_instances.len();
        if count == 0 {
            writeln!(
                writer,
                "{}\t n/a \t n/a",
                cluster.split('/').nth(1).unwrap().to_owned()
            )?;
            continue;
        }

        if let Some(first) = ecs
            .describe_container_instances(DescribeContainerInstancesRequest {
                cluster: Some(cluster.clone()),
                container_instances,
                ..DescribeContainerInstancesRequest::default()
            })
            .sync()?
            .container_instances
            .unwrap_or_default()
            .iter()
            .next()
        {
            if let Some(instance_type) = first
                .attributes
                .clone()
                .unwrap_or_default()
                .iter()
                .find_map(|attr| match attr {
                    Attribute { name, value, .. } if name == "ecs.instance-type" => value.clone(),
                    _ => None,
                })
            {
                writeln!(
                    writer,
                    "{}\t({} x {})\t~${:.*} monthly",
                    cluster.split('/').nth(1).unwrap().to_owned(),
                    count,
                    instance_type,
                    2,
                    INSTANCE_HOURLY
                        .get(instance_type.as_str())
                        .map(|hourly| (hourly * count as f32) * 24.0 * 7.0 * 4.0)
                        .unwrap_or_default()
                )?;
            }
        }
    }
    writer.flush()?;
    Ok(())
}
