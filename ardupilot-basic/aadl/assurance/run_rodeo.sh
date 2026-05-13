#! /bin/bash

rust-rodeo-client --hamr-report-filepath $HAMR_ATTESTATION_ROOT/sysml_attestation_report.json -s $RODEO_ROOT/rodeo_configs/sessions/session_union.json -m $RODEO_ROOT/testing/manifests/Manifest_P0.json -o $RODEO_ROOT/testing/outputs/ -p $HAMR_ATTESTATION_ROOT/hamr_maestro_golden_evidence.json
rust-rodeo-client -t $HAMR_ATTESTATION_ROOT/hamr_maestro_term.json -s $RODEO_ROOT/rodeo_configs/sessions/session_union.json -m $RODEO_ROOT/testing/manifests/Manifest_P0.json -o $RODEO_ROOT/testing/outputs/ -a
cp $HAMR_ATTESTATION_ROOT/appsumm_response.json "$1"
