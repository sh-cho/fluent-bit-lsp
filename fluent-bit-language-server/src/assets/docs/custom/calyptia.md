# Calyptia

Calyptia custom plugin

[//]: # (This documentation is handwritten by Seonghyeon Cho. There's no official documentation for this plugin I suppose)

## Configuration Parameters

| Key                 | Description                                               | Default                |
|:--------------------|:----------------------------------------------------------|:-----------------------|
| api_key             | Calyptia Cloud API Key.                                   |                        |
| store_path          |                                                           |                        |
| calyptia_host       |                                                           | cloud-api.calyptia.com |
| calyptia_port       |                                                           | 443                    |
| calyptia_tls        |                                                           | true                   |
| calyptia_tls.verify |                                                           | true                   |
| add_label           | Label to append to the generated metric.                  |                        |
| machine_id          | Custom machine_id to be used when registering agent       |                        |
| fleet_id            | Fleet id to be used when registering agent in a fleet     |                        |
| fleet.config_dir    | Base path for the configuration directory.                |                        |
| fleet.interval_sec  | Set the collector interval                                | -1                     |
| fleet.interval_nsec | Set the collector interval (nanoseconds)                  | -1                     |
| fleet_name          | Fleet name to be used when registering agent in a fleet   |                        |
| pipeline_id         | Pipeline ID for reporting to calyptia cloud.              |                        |
