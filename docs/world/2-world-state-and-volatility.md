# World State and Volatility

Device logic values are external world state. Reading a device field is not the
same kind of operation as reading a local variable or a StationC-owned RAM cell.

## Device Reads Are Observations

Every `device_read` is a world-facing observation at the moment that IC10
instruction executes. A repeated read may observe a different value.

Suspicious:

```c
if (device_read(sensor, Temperature) > 300 &&
    device_read(sensor, Temperature) < 310)
{
    device_write(cooler, On, 1);
}
```

That code reads `Temperature` twice. The second comparison is not guaranteed to
use the same value as the first comparison.

Use a local sample when the program needs one coherent value:

```c
float temperature = device_read(sensor, Temperature);

if (temperature > 300 && temperature < 310)
{
    device_write(cooler, On, 1);
}
```

The local `temperature` is ordinary StationC state. It remains stable until the
program assigns to it.

## Compiler Rule

World-facing operations are effectful. The compiler must not treat these calls
like ordinary pure expressions:

```c
device_read(device, field)
batch_read(group, field, mode)
device_write(device, field, value)
batch_write(group, field, value)
```

The compiler must not:

- remove them,
- combine repeated reads,
- hoist them out of control flow,
- move them across other world operations,
- move them across tick or yield boundaries.

If code wants a stable value, it must store the result in a local, temporary, or
runtime-owned snapshot buffer.

## Snapshots

StationC may provide explicit snapshot helpers later:

```c
DeviceSnapshot gas = device_sample(sensor);

if (gas.temperature > 300 && gas.temperature < 310)
{
    device_write(cooler, On, 1);
}
```

That kind of API would make the sampling boundary visible in source code and
visible to simulator diagnostics.
