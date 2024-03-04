# netcheck

Netcheck is a tool to check the availability of a group of network connections.

Exposes metrics for Prometheus so can be used to monitor the availability of a network and alert (via alertmanager) when it is not available.

## Use cases

- Check if a list of networks/services are available.
- Check if an internal network can reach an external network (This was used internally to check if a kubernetes cluster and istio mesh can readily and repeatability
  reach an external network and alert when it can not).
- Check if a network is available from a specific location.
- Simple pingdom/uptime alternative. 
- This is **not** intended to replace a full monitoring solution, but as part of your network availability tooling.
- This is **not** intended to be used as a load testing (ie ab, siege, wrk etc) tool.

## Usage

Suggested usage is to run this as a docker container and use the provided chart to deploy to kubernetes.

For local usage you can run the following:

```shell
./netcheck run --help
Runs the netcheck service and checks the network using the passed targets

Usage: cli run [OPTIONS]

Options:
  -D, --debug <DEBUG>
          [possible values: true, false]

  -t, --target <TARGET>
          List of targets to check if a network connection is attainable
          
          [default: external=https://one.one.one.one,https://dns.google]

      --connect <CONNECT_TIMEOUT_MS>
          Connect timeout milliseconds to be considered a failure
          
          [default: 500]

  -v, --verbose <VERBOSE>
          [possible values: true, false]

  -l, --log-level <LOG_LEVEL>
          

      --timeout <TIMEOUT_MS>
          Timeout milliseconds to be considered a failure
          
          [default: 500]

  -w, --wait <WAIT_TIME_SECONDS>
          Time to wait between requests in seconds
          
          [default: 2]

      --failure-threshold <FAILURE_THRESHOLD>
          Failures in a row to determine if target is failing
          
          [default: 5]

  -h, --help
          Print help (see a summary with '-h')
```

Example, targetting an external network, and an internal network:
```shell
./netcheck run --target external=https://one.one.one.one,https://dns.google --target internal=http://hellosvc.test.svc.cluster.local:9111,http://hello2svc.test.svc.cluster.local:9111
```
