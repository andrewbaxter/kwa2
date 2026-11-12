use {
    infra_flatcar::{
        Container,
        ContentKind,
        Host,
        FLATCAR_IMAGE,
    },
    serde_json::json,
    std::{
        collections::HashMap,
        env,
        fs,
        path::{
            Path,
            PathBuf,
        },
    },
    terrars::{
        primvec,
        BuildOutput,
        BuildStack,
        BuildVariable,
        PrimExpr,
        PrimField,
        Stack,
    },
    terrars_andrewbaxter_dinker::{
        BuildImage,
        BuildImage_Build,
        BuildImage_Files,
        BuildProviderDinker,
        Image,
    },
    terrars_dnsimple_dnsimple::{
        BuildProviderDnsimple,
        BuildZoneRecord,
    },
    terrars_hashicorp_google::{
        BuildArtifactRegistryRepository,
        ArtifactRegistryRepository,
        ArtifactRegistryRepositoryIamBinding,
        BuildArtifactRegistryRepositoryIamBinding,
        BuildArtifactRegistryRepository_CleanupPolicies,
        BuildArtifactRegistryRepository_CleanupPolicies_Condition,
        BuildComputeAddress,
        BuildComputeDisk,
        BuildComputeFirewall,
        BuildComputeFirewall_Allow,
        BuildComputeInstance,
        BuildComputeInstance_AttachedDisk,
        BuildComputeInstance_BootDisk,
        BuildComputeInstance_BootDisk_InitializeParams,
        BuildComputeInstance_NetworkInterface,
        BuildComputeInstance_NetworkInterface_AccessConfig,
        BuildComputeInstance_NetworkInterface_Ipv6AccessConfig,
        BuildComputeInstance_ServiceAccount,
        BuildComputeNetwork,
        BuildComputeSubnetwork,
        BuildProjectService,
        BuildProviderGoogle,
        BuildSecretManagerSecret,
        BuildSecretManagerSecretIamMember,
        BuildSecretManagerSecretVersion,
        BuildSecretManagerSecret_Replication,
        BuildSecretManagerSecret_Replication_Auto,
        BuildServiceAccount,
        BuildServiceAccountKey,
    },
    terrars_hashicorp_random::{
        BuildId,
        BuildPassword,
        BuildProviderRandom,
    },
};

const SERVER_COUNT: usize = 2;
const REGISTRY_ID: &str = "kwa-test";
const USER: &str = "test";
const PERSISTENT_DIR: &str = "/mnt/persistent";
const CONFIG_DIR: &str = "/etc/kwa-test";
const FDAP_ADDR: &str = "[::1]:64116";
const FDAP_LOGIN_ADDR: &str = "[::1]:8001";
const KWA_ADDR: &str = "127.0.0.1:8080";

struct ContainerImage {
    name: &'static str,
    tf_id: &'static str,
    sources: Vec<&'static str>,
    extra_bins: Vec<&'static str>,
}

fn build_image(
    stack: &mut Stack,
    root: &Path,
    stage_root: &Path,
    registry: &ArtifactRegistryRepository,
    registry_host: &str,
    project: &str,
    push_binding: &ArtifactRegistryRepositoryIamBinding,
    push_key: &PrimExpr<String>,
    spec: &ContainerImage,
) -> Image {
    let out = stage_root.join("nix").join(spec.name);
    let bin_dest = format!("/{}", spec.name);
    return BuildImage {
        tf_id: spec.tf_id.into(),
        dest: format!(
            "docker://{}/{}/{}/{}:latest",
            registry_host,
            project,
            registry.repository_id(),
            spec.name
        ).into(),
        files: [
            BuildImage_Files { source: out.join(spec.name).to_string_lossy().to_string().into() }
                .build()
                .set_dest(&bin_dest)
                .set_mode("0755"),
            BuildImage_Files { source: out.join("ca-certificates.crt").to_string_lossy().to_string().into() }
                .build()
                .set_dest("/etc/ssl/certs/ca-certificates.crt")
                .set_mode("0644"),
        ]
            .into_iter()
            .chain(
                spec
                    .extra_bins
                    .iter()
                    .map(
                        |b| BuildImage_Files { source: out.join(b).to_string_lossy().to_string().into() }
                            .build()
                            .set_dest(&format!("/{}", b))
                            .set_mode("0755"),
                    ),
            )
            .collect(),
    }
        .build(stack)
        .depends_on(push_binding)
        .set_build(BuildImage_Build {
            command: primvec![
                "nix",
                "--extra-experimental-features",
                "nix-command",
                "build",
                "--print-build-logs",
                "-f",
                "kwa3/source/infra/test/containers.nix",
                spec.name,
                "-o",
                &out.to_string_lossy().to_string()
            ].into(),
            sources: spec.sources.iter().map(|s| PrimField::from(s.to_string())).collect::<Vec<_>>().into(),
        }.build().set_dir(root.to_str().unwrap()))
        .set_arch("amd64")
        .set_os("linux")
        .set_entrypoint(primvec![bin_dest])
        .set_dest_user("_json_key_base64")
        .set_dest_password(push_key);
}

fn main() {
    let root = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).canonicalize().unwrap();
    let workspace_root = root.parent().unwrap().parent().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    let stage_root = root.join("stage");
    let tf_root = stage_root.join("tf");
    fs::create_dir_all(&tf_root).unwrap();
    let stack = &mut BuildStack {}.build();
    let domain_zone = "isandrew.com";
    let location = "us-west1";
    let zone = format!("{}-b", location);
    let project = "testing-513313";
    let registry_host = format!("{}-docker.pkg.dev", location);
    let dnsimple_token = &BuildVariable { tf_id: "dnsimple_token".into() }.build(stack).set_sensitive(true);
    let google_access_token =
        &BuildVariable { tf_id: "google_access_token".into() }.build(stack).set_sensitive(true);
    BuildProviderRandom {}.build(stack);
    BuildProviderDinker {}
        .build(stack)
        .set_cache_dir(stage_root.join("dinker_cache").to_string_lossy().to_string());
    BuildProviderDnsimple {}.build(stack).set_account("139852").set_token(dnsimple_token);
    BuildProviderGoogle {}.build(stack).set_access_token(google_access_token).set_project(project);
    let enable_gcp_resource_api = BuildProjectService {
        tf_id: "zXUM1OZ7T".into(),
        service: "cloudresourcemanager.googleapis.com".into(),
    }.build(stack).set_disable_on_destroy(false);
    let enable_gcp_compute_api = BuildProjectService {
        tf_id: "zLYA6BYTH".into(),
        service: "compute.googleapis.com".into(),
    }.build(stack).set_disable_on_destroy(false).depends_on(&enable_gcp_resource_api);
    let enable_gcp_iam_api = BuildProjectService {
        tf_id: "zKT1AMQ4P".into(),
        service: "iam.googleapis.com".into(),
    }.build(stack).set_disable_on_destroy(false).depends_on(&enable_gcp_resource_api);
    let enable_artifact_registry_api = BuildProjectService {
        tf_id: "zKTAR7V2N".into(),
        service: "artifactregistry.googleapis.com".into(),
    }.build(stack).set_disable_on_destroy(false).depends_on(&enable_gcp_resource_api);
    let enable_secret_manager_api = BuildProjectService {
        tf_id: "zKTSM4R8V".into(),
        service: "secretmanager.googleapis.com".into(),
    }.build(stack).set_disable_on_destroy(false).depends_on(&enable_gcp_resource_api);

    // Images
    let registry =
        BuildArtifactRegistryRepository {
            tf_id: "zKTREG5WX".into(),
            repository_id: REGISTRY_ID.into(),
            format: "DOCKER".into(),
        }
            .build(stack)
            .depends_on(&enable_artifact_registry_api)
            .set_location(location)
            .set_cleanup_policy_dry_run(false)
            .set_cleanup_policies(
                vec![
                    BuildArtifactRegistryRepository_CleanupPolicies { id: "delete-untagged".into() }
                        .build()
                        .set_action("DELETE")
                        .set_condition(
                            vec![
                                BuildArtifactRegistryRepository_CleanupPolicies_Condition {}
                                    .build()
                                    .set_tag_state("UNTAGGED"),
                            ],
                        ),
                ],
            );
    let registry_path = format!("{}/{}/{}", registry_host, project, REGISTRY_ID);
    let push_account = BuildServiceAccount {
        tf_id: "zKTPUSH3A".into(),
        account_id: "kwa-test-push".into(),
    }.build(stack).depends_on(&enable_gcp_iam_api);
    let push_binding = BuildArtifactRegistryRepositoryIamBinding {
        tf_id: "zKTPUSH3B".into(),
        members: vec![format!("serviceAccount:{}", push_account.email()).into()].into(),
        repository: registry.id().into(),
        role: "roles/artifactregistry.writer".into(),
    }.build(stack);
    let push_key = BuildServiceAccountKey {
        tf_id: "zKTPUSH3K".into(),
        service_account_id: push_account.id().into(),
    }.build(stack).private_key();
    let host_accounts = (1 ..= SERVER_COUNT).map(|i| BuildServiceAccount {
        tf_id: format!("zKTHOST8A-{}", i),
        account_id: format!("kwa-test-host-{}", i).into(),
    }.build(stack).depends_on(&enable_gcp_iam_api)).collect::<Vec<_>>();
    BuildArtifactRegistryRepositoryIamBinding {
        tf_id: "zKTHOST8B".into(),
        members: host_accounts
            .iter()
            .map(|a| PrimField::from(format!("serviceAccount:{}", a.email())))
            .collect::<Vec<_>>()
            .into(),
        repository: registry.id().into(),
        role: "roles/artifactregistry.reader".into(),
    }.build(stack);
    let image_kwa =
        build_image(
            stack,
            &workspace_root,
            &stage_root,
            &registry,
            &registry_host,
            project,
            &push_binding,
            &push_key,
            &ContainerImage {
                name: "kwa",
                tf_id: "zKTIMGKWA",
                extra_bins: vec!["ffmpeg"],
                sources: vec![
                    "kwa3/source/build.nix",
                    "kwa3/source/infra/test/containers.nix",
                    "kwa3/source/native/**",
                    "kwa3/source/native-bindgen/**",
                    "kwa3/source/nixbuild/**",
                    "kwa3/source/shared/**",
                    "kwa3/source/spaghettinuum/source/native/**",
                    "kwa3/source/spaghettinuum/source/shared/**",
                    "kwa3/source/wasm/**",
                    "!**/target/**",
                ],
            },
        );
    let image_openfdap =
        build_image(
            stack,
            &workspace_root,
            &stage_root,
            &registry,
            &registry_host,
            project,
            &push_binding,
            &push_key,
            &ContainerImage {
                name: "openfdap",
                tf_id: "zKTIMGOFD",
                extra_bins: vec![],
                sources: vec!["kwa3/source/infra/test/containers.nix", "openfdap/source/**", "!**/target/**"],
            },
        );
    let image_fdap_login =
        build_image(
            stack,
            &workspace_root,
            &stage_root,
            &registry,
            &registry_host,
            project,
            &push_binding,
            &push_key,
            &ContainerImage {
                name: "fdap-login",
                tf_id: "zKTIMGFDL",
                extra_bins: vec![],
                sources: vec!["kwa3/source/infra/test/containers.nix", "fdap-login/source/**", "!**/target/**"],
            },
        );

    // Login
    let password = BuildPassword {
        tf_id: "zWHDDC76S".into(),
        length: 20f64.into(),
    }.build(stack).set_special(false);
    let password_hash = password.bcrypt_hash();

    // Network
    let net = BuildComputeNetwork {
        tf_id: "zGFWM4QBO".into(),
        name: "kwa-test-net".into(),
    }.build(stack).depends_on(&enable_gcp_compute_api).set_auto_create_subnetworks(false);
    let subnet = {
        let id = "zWS8KL6HM";
        BuildComputeSubnetwork {
            tf_id: id.into(),
            name: id.to_ascii_lowercase().into(),
            network: net.id().into(),
        }
            .build(stack)
            .set_ip_cidr_range("10.0.0.0/24")
            .set_region(location)
            .set_stack_type("IPV4_IPV6")
            .set_ipv6_access_type("EXTERNAL")
    };
    for (id, source_range) in [("zDW3MPB9F", "0.0.0.0/0"), ("zLOM2QQ3F", "::/0")] {
        BuildComputeFirewall {
            tf_id: id.into(),
            network: net.id().into(),
            name: id.to_ascii_lowercase().into(),
        }
            .build(stack)
            .set_priority(1000f64)
            .set_direction("INGRESS")
            .set_source_ranges(primvec![source_range])
            .set_allow(
                vec![
                    BuildComputeFirewall_Allow { protocol: "tcp".into() }
                        .build()
                        .set_ports(primvec!["80", "443", "48390", "12434", "21225"]),
                ],
            );
    }

    // Servers
    for i in 1 ..= SERVER_COUNT {
        let name = format!("kwa-test-{}", i);
        let host_account = &host_accounts[i - 1];
        let login_name = format!("login.{}", name);
        let domain = format!("{}.{}", name, domain_zone);
        let login_domain = format!("{}.{}", login_name, domain_zone);
        let ipv4 = BuildComputeAddress {
            tf_id: format!("zKT4V2M9Q-{}", i),
            name: format!("{}-ipv4", name).into(),
        }.build(stack).depends_on(&enable_gcp_compute_api).set_region(location).set_ip_version("IPV4");
        let ipv6 =
            BuildComputeAddress {
                tf_id: format!("zKT6P8W3R-{}", i),
                name: format!("{}-ipv6", name).into(),
            }
                .build(stack)
                .set_region(location)
                .set_subnetwork(&subnet.id())
                .set_ip_version("IPV6")
                .set_ipv6_endpoint_type("VM");
        let disk = BuildComputeDisk {
            tf_id: format!("zKTD5N1XS-{}", i),
            name: format!("{}-data", name).into(),
        }.build(stack).depends_on(&enable_gcp_compute_api).set_zone(&zone).set_size(10f64);
        let mut token = |tf_id: &str| BuildPassword {
            tf_id: format!("{}-{}", tf_id, i),
            length: 48f64.into(),
        }.build(stack).set_special(false).result();
        let root_token = token("zKTRT9Q2M");
        let login_token = token("zKTLG4H7X");
        let kwa_token = token("zKTKW8N3D");
        let mut secret = |tf_id: &str, secret_name: &str, value: &PrimExpr<String>| {
            let secret =
                BuildSecretManagerSecret {
                    tf_id: format!("{}-{}", tf_id, i),
                    secret_id: format!("{}-{}", name, secret_name).into(),
                }
                    .build(stack)
                    .depends_on(&enable_secret_manager_api)
                    .set_replication(
                        vec![
                            BuildSecretManagerSecret_Replication {}
                                .build()
                                .set_auto(vec![BuildSecretManagerSecret_Replication_Auto {}.build()]),
                        ],
                    );
            BuildSecretManagerSecretIamMember {
                tf_id: format!("{}-access-{}", tf_id, i),
                member: format!("serviceAccount:{}", host_account.email()).into(),
                role: "roles/secretmanager.secretAccessor".into(),
                secret_id: secret.id().into(),
            }.build(stack);
            return BuildSecretManagerSecretVersion {
                tf_id: format!("{}-version-{}", tf_id, i),
                secret: secret.id().into(),
            }.build(stack).set_secret_data(value).name();
        };
        let root_token_secret = secret("zKTSRT5W1", "fdap-root-token", &root_token);
        let login_token_secret = secret("zKTSLG2P6", "fdap-login-token", &login_token);
        let kwa_token_secret = secret("zKTSKW7J3", "kwa-fdap-token", &kwa_token);
        let mut ign = Host::new(&registry_host);
        ign.journal_to_serial_console();
        let mut token_sha256 = |t: &PrimExpr<String>| {
            let e: PrimExpr<String> = stack.func("sha256").a(stack.expr::<String>(t.raw())).into();
            return ign.interp(&e);
        };
        let root_token_sha256 = token_sha256(&root_token);
        let login_token_sha256 = token_sha256(&login_token);
        let kwa_token_sha256 = token_sha256(&kwa_token);
        let root_token_secret = ign.interp(&root_token_secret);
        let login_token_secret = ign.interp(&login_token_secret);
        let kwa_token_secret = ign.interp(&kwa_token_secret);
        let password_hash = ign.interp(&password_hash);
        let ipv4_addr = ign.interp(&ipv4.address());
        let ipv6_addr = ign.interp(&ipv6.address());
        let image_kwa = format!("{}/kwa@{}", registry_path, ign.interp(&image_kwa.hash()));
        let image_openfdap = format!("{}/openfdap@{}", registry_path, ign.interp(&image_openfdap.hash()));
        let image_fdap_login = format!("{}/fdap-login@{}", registry_path, ign.interp(&image_fdap_login.hash()));
        let mount = ign.persistent_disk("persistent", "3d02cfd4-968a-4fe4-a2a0-fe84614485f6", PERSISTENT_DIR);
        let dirs =
            ign.oneshot(
                "kwa-test-dirs",
                &[&mount],
                &[],
                &[],
                &[],
                &[
                    "/usr/bin/mkdir",
                    "-p",
                    &format!("{}/caddy/data", PERSISTENT_DIR),
                    &format!("{}/caddy/config", PERSISTENT_DIR),
                    &format!("{}/openfdap", PERSISTENT_DIR),
                    &format!("{}/fdap-login", PERSISTENT_DIR),
                    &format!("{}/kwa/cache", PERSISTENT_DIR),
                    &format!("{}/kwa/persistent", PERSISTENT_DIR),
                ],
            );
        let config_path = |n: &str| format!("{}/{}", CONFIG_DIR, n);
        let persistent_mount = format!("{}:{}", PERSISTENT_DIR, PERSISTENT_DIR);
        let config_mount = format!("{}:{}:ro", CONFIG_DIR, CONFIG_DIR);
        let secrets_path = |n: &str| format!("/etc/kwa-test-secrets/{}", n);
        let seed_env = secrets_path("kwa-test-seed.env");
        let fdap_login_env = secrets_path("fdap-login.env");
        let kwa_env = secrets_path("kwa.env");
        ign.env_file(&seed_env, 0o600, &[("FDAP_TOKEN", &root_token_secret)]);
        ign.env_file(&fdap_login_env, 0o600, &[("FDAP_TOKEN", &login_token_secret)]);
        ign.env_file(&kwa_env, 0o600, &[("FDAP_TOKEN", &kwa_token_secret)]);
        ign.file(&config_path("openfdap.json"), 0o600, &serde_json::to_string(&json!({
            "bind_addr": FDAP_ADDR,
            "data_dir": format !("{}/openfdap", PERSISTENT_DIR),
            "users":[{
                "token": {
                    "sha256": root_token_sha256
                },
                "access":[{
                    "path":[],
                    "action": {
                        "read": true,
                        "write": true
                    }
                }]
            }, {
                "token": {
                    "sha256": login_token_sha256
                },
                "access":[{
                    "path":[{
                        "string": "user"
                    }, "wildcard", {
                        "string": "fdap-login"
                    }],
                    "action": {
                        "read": true,
                        "write": false
                    }
                }, {
                    "path":[{
                        "string": "fdap-login"
                    }, {
                        "string": "clients"
                    }, "wildcard"],
                    "action": {
                        "read": true,
                        "write": false
                    }
                }]
            }, {
                "token": {
                    "sha256": kwa_token_sha256
                },
                "access":[{
                    "path":[{
                        "string": "user"
                    }, "wildcard", {
                        "string": "kwa"
                    }],
                    "action": {
                        "read": true,
                        "write": false
                    }
                }]
            }]
        })).unwrap(), ContentKind::Json);
        ign.file(&config_path("fdap_seed.json"), 0o600, &serde_json::to_string(&json!({
            "user": {
                USER: {
                    "fdap-login": {
                        "password": password_hash
                    },
                    "kwa": {
                    }
                }
            },
            "fdap-login": {
                "clients": {
                    "kwa": {
                        "redirect_uris":[format!("https://{}/oidc", domain)]
                    }
                }
            }
        })).unwrap(), ContentKind::Json);
        ign.file(&config_path("fdap-login.json"), 0o600, &serde_json::to_string(&json!({
            "bind_addr": FDAP_LOGIN_ADDR,
            "public_base_url": format !("https://{}", login_domain),
            "fdap_base_url": format !("http://{}", FDAP_ADDR),
            "trusted_proxies":["::1/128"],
            "jwt_key_path": format !("{}/fdap-login/jwt.pem", PERSISTENT_DIR)
        })).unwrap(), ContentKind::Json);
        ign.file(&config_path("kwa.json"), 0o600, &serde_json::to_string(&json!({
            "public_url": format !("https://{}", domain),
            "bind_sockaddr": KWA_ADDR,
            "spagh_node_bind_sockaddr": "[::]:48390",
            "spagh_publisher_bind_sockaddr": "[::]:12434",
            "spagh_publisher_advertise_port": null,
            "s2s_bind_sockaddr": "[::]:21225",
            "s2s_advertise_port": null,
            "global_addrs":[{
                "fixed": ipv6_addr
            }, {
                "fixed": ipv4_addr
            }],
            "cache_dir": format !("{}/kwa/cache", PERSISTENT_DIR),
            "persistent_dir": format !("{}/kwa/persistent", PERSISTENT_DIR),
            "ffmpeg": "/ffmpeg",
            "oidc_config": {
                "provider_url": format !("https://{}", login_domain),
                "client_id": "kwa",
                "client_secret": null
            },
            "users": {
                "fdap":["kwa"]
            },
            "fdap": {
                "url": format !("http://{}", FDAP_ADDR)
            }
        })).unwrap(), ContentKind::Json);
        ign.file(
            &config_path("Caddyfile"),
            0o644,
            &format!(
                "{} {{\n\treverse_proxy {}\n}}\n\n{} {{\n\treverse_proxy {}\n}}\n",
                domain,
                KWA_ADDR,
                login_domain,
                FDAP_LOGIN_ADDR
            ),
            ContentKind::Raw,
        );
        let seed_script = "/opt/bin/kwa-test-seed";
        ign.file(
            seed_script,
            0o755,
            &format!(
                "#!/bin/bash\nset -eu -o pipefail\n/usr/bin/curl -sSf -g --retry 30 --retry-connrefused --retry-delay 1 -X POST -H \"Authorization: Bearer $FDAP_TOKEN\" -H 'Content-Type: application/json' --data @{} 'http://{}/'\n",
                config_path("fdap_seed.json"),
                FDAP_ADDR
            ),
            ContentKind::Raw,
        );
        let openfdap = ign.container(Container {
            name: "openfdap",
            image: &image_openfdap,
            requires: &[&dirs],
            after: &[],
            env_files: &[],
            docker_args: &["-v", &persistent_mount, "-v", &config_mount],
            args: &["serve", "--config", &config_path("openfdap.json")],
            restart_on: &[&config_path("openfdap.json")],
        });
        let seed =
            ign.oneshot(
                "kwa-test-seed",
                &[&openfdap],
                &[],
                &[&seed_env],
                &[seed_script, &seed_env, &config_path("fdap_seed.json")],
                &[seed_script],
            );
        let fdap_login = ign.container(Container {
            name: "fdap-login",
            image: &image_fdap_login,
            requires: &[&seed],
            after: &[],
            env_files: &[&fdap_login_env],
            docker_args: &["-v", &persistent_mount, "-v", &config_mount],
            args: &["run", &config_path("fdap-login.json")],
            restart_on: &[&config_path("fdap-login.json"), &fdap_login_env],
        });
        let caddy = ign.container(Container {
            name: "caddy",
            image: "docker.io/library/caddy:2.11.7@sha256:36c24ad91a7ff7bc4c87fe22bfa8df3c4b18da0773d73fc12ee2c52b98c0bb7e",
            requires: &[&dirs],
            after: &[],
            env_files: &[],
            docker_args: &[
                "-v",
                &format!("{}:/etc/caddy/Caddyfile:ro", config_path("Caddyfile")),
                "-v",
                &format!("{}/caddy/data:/data", PERSISTENT_DIR),
                "-v",
                &format!("{}/caddy/config:/config", PERSISTENT_DIR),
            ],
            args: &[],
            restart_on: &[&config_path("Caddyfile")],
        });
        ign.container(Container {
            name: "kwa",
            image: &image_kwa,
            requires: &[&seed],
            after: &[&fdap_login, &caddy],
            env_files: &[&kwa_env],
            docker_args: &["-v", &persistent_mount, "-v", &config_mount],
            args: &[&config_path("kwa.json")],
            restart_on: &[&config_path("kwa.json"), &kwa_env],
        });
        let host_config = ign.build(stack);
        let user_data = host_config.user_data;
        let user_data_sha_arg: PrimExpr<String> = stack.expr(user_data.raw());
        let user_data_version =
            BuildId {
                tf_id: format!("zKTUDV6C1-{}", i),
                byte_length: 8f64.into(),
            }
                .build(stack)
                .set_keepers(
                    HashMap::from(
                        [
                            (
                                "user_data".to_string(),
                                Into::<PrimField<String>>::into(stack.func("sha256").a(user_data_sha_arg)),
                            ),
                        ],
                    ),
                );
        BuildComputeInstance {
            tf_id: format!("zKTI7H4LB-{}", i),
            machine_type: "e2-micro".into(),
            name: name.clone().into(),
        }
            .build(stack)
            .set_zone(&zone)
            .replace_triggered_by_resource(&user_data_version)
            .set_metadata(
                HashMap::from(
                    [
                        ("user-data".to_string(), Into::<PrimField<String>>::into(user_data)),
                        ("agent-spec".to_string(), Into::<PrimField<String>>::into(host_config.agent_spec)),
                        ("serial-port-logging-enable".to_string(), Into::<PrimField<String>>::into("TRUE")),
                    ],
                ),
            )
            .set_service_account(
                vec![
                    BuildComputeInstance_ServiceAccount { scopes: primvec!["cloud-platform"].into() }
                        .build()
                        .set_email(host_account.email()),
                ],
            )
            .set_boot_disk(
                vec![
                    BuildComputeInstance_BootDisk {}
                        .build()
                        .set_initialize_params(
                            vec![BuildComputeInstance_BootDisk_InitializeParams {}.build().set_image(FLATCAR_IMAGE)],
                        ),
                ],
            )
            .set_attached_disk(
                vec![
                    BuildComputeInstance_AttachedDisk { source: disk.self_link().into() }
                        .build()
                        .set_device_name("persistent"),
                ],
            )
            .set_network_interface(
                vec![
                    BuildComputeInstance_NetworkInterface {}
                        .build()
                        .set_subnetwork(subnet.id())
                        .set_stack_type("IPV4_IPV6")
                        .set_access_config(
                            vec![
                                BuildComputeInstance_NetworkInterface_AccessConfig {}
                                    .build()
                                    .set_nat_ip(ipv4.address()),
                            ],
                        )
                        .set_ipv6_access_config(
                            vec![
                                BuildComputeInstance_NetworkInterface_Ipv6AccessConfig {
                                    network_tier: "PREMIUM".into(),
                                }
                                    .build()
                                    .set_external_ipv6(ipv6.address())
                                    .set_external_ipv6_prefix_length(ipv6.prefix_length().to_string()),
                            ],
                        ),
                ],
            );
        for (rel_name, rel_id) in [(&name, "main"), (&login_name, "login")] {
            BuildZoneRecord {
                tf_id: format!("zKTRA6C2E-{}-{}", rel_id, i),
                name: rel_name.as_str().into(),
                zone_name: domain_zone.into(),
                type_: "A".into(),
                value: ipv4.address().into(),
            }.build(stack).set_ttl(180f64);
            BuildZoneRecord {
                tf_id: format!("zKTRQ9F5J-{}-{}", rel_id, i),
                name: rel_name.as_str().into(),
                zone_name: domain_zone.into(),
                type_: "AAAA".into(),
                value: ipv6.address().into(),
            }.build(stack).set_ttl(180f64);
        }
        BuildOutput::<String> {
            tf_id: format!("url_{}", i),
            value: format!("https://{}/", domain).into(),
        }.build(stack);
    }
    BuildOutput::<String> {
        tf_id: "user".into(),
        value: USER.into(),
    }.build(stack);
    BuildOutput::<String> {
        tf_id: "password".into(),
        value: stack.func("nonsensitive").a(password.result()).into(),
    }.build(stack);
    fs::write(tf_root.join("stack.tf.json"), stack.serialize(&tf_root.join("state.json")).unwrap()).unwrap();
}
