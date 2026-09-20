# Vita ADB — transporte USB de depuración

`vita-adb` es un equivalente limitado y explícito de ADB para una PS Vita con
HENkaku/taiHEN. No ejecuta comandos arbitrarios ni abre un shell del sistema:
ofrece un transporte USB autenticable para diagnóstico y una lista cerrada de
comandos. La primera versión implementa `ping` e `info`; la integración de
OpenNOW se añadirá después, sobre este transporte ya verificado.

## Qué hace y qué no hace

- Usa `SceUsbSerialForDriver` desde un plugin de kernel (`.skprx`). USB es el
  transporte real, no Wi-Fi ni GitHub.
- El plugin sólo se carga si la Vita ya tiene HENkaku/taiHEN. Una Vita oficial
  no puede cargar plugins de kernel de terceros.
- Al activarse toma el controlador USB de la Vita; Content Manager y VitaShell
  USB no pueden estar activos al mismo tiempo.
- No instala VPKs, no modifica archivos, no enumera secretos y no acepta una
  consola arbitraria. Es una base segura de diagnóstico, no una puerta trasera.

## Compilar

Desde la raíz de OpenNOW Vita:

```powershell
docker run --rm -v "${PWD}:/work" -w /work opennow-build sh -lc `
  'cmake -S vita-adb/kernel -B /tmp/vita-adb-build && cmake --build /tmp/vita-adb-build'
```

El resultado es `/tmp/vita-adb-build/vita-adb.skprx`; cópialo a
`ur0:tai/vita-adb.skprx` con VitaShell y añade esta única línea bajo `*KERNEL`
en una copia de seguridad de `ur0:tai/config.txt`:

```text
ur0:tai/vita-adb.skprx
```

Reinicia la Vita. Para volver atrás, elimina esa línea y el archivo `.skprx`.

## PC

Instala `pyserial` y usa el puerto que Windows asigne al dispositivo `PS Vita
Type D`:

```powershell
py -m pip install -r vita-adb/host/requirements.txt
py vita-adb/host/vita_adb.py devices
py vita-adb/host/vita_adb.py --port COM7 ping
py vita-adb/host/vita_adb.py --port COM7 info
```

Si Windows no asigna un puerto COM, no se debe sustituir ningún controlador a
ciegas. Se registra primero el VID/PID y se prepara un controlador WinUSB
específico y reversible.

## Protocolo

Cada trama usa `VAD1`, versión 1, operación de un byte y tamaño `u16`
little-endian. El tamaño máximo es 1024 bytes. Operaciones actuales: `PING`
(`0x01`) e `INFO` (`0x02`). Las respuestas usan el bit alto y los errores
`0xFF`. Los mensajes desconocidos se rechazan: no se interpretan como código.
