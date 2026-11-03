#!/usr/bin/env bash
set -xeu -o pipefail
action="${1:-apply}"
case "$action" in
	apply|destroy) ;;
	*) echo "usage: $0 [apply|destroy]" >&2; exit 1 ;;
esac
cd "$(dirname "$0")"
cargo run
(
	cd stage/tf
	terraform init -input=false
	set +x
	TF_VAR_google_access_token="$(gcloud auth print-access-token)"
	export TF_VAR_google_access_token
	TF_VAR_dnsimple_token="$(pw read /device/apricorn/container/dev/dnsimple/token)"
	export TF_VAR_dnsimple_token
	set -x
	if [ "$action" = apply ]; then
		terraform apply -auto-approve
	else
		terraform "$action"
	fi
)
