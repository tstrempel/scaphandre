# Scaphandre

- fork of https://github.com/hubblo-org/scaphandre
- applied patches and modifications to add timestamps, average load, CPU load, CPU temperature, total memory, free memory

## Replication package

```
git clone https://github.com/tstrempel/scaphandre
cd scaphandre
docker build -t strempel/scaphandre - < Dockerfile
docker run -v /sys/class/powercap:/sys/class/powercap -v /proc:/proc -ti strempel/scaphandre json -t 10
```
