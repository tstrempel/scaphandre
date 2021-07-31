# Scaphandre

- fork of https://github.com/hubblo-org/scaphandre
- applied patches and modifications to add timestamps, average load, CPU load, CPU temperature, total memory, free memory

## Replication package

```
git clone https://github.com/tstrempel/scaphandre
cd scaphandre
docker build -t tstrempel/scaphandre - < Dockerfile
docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -ti tstrempel/scaphandre --no-header json --timeout 10 --step 1 --step_nano 0 --max-top-consumers=10
```
