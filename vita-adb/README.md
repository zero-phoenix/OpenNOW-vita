# vita-adbd — diagnóstico controlado para PS Vita

`vita-adbd` es un equivalente deliberadamente limitado de ADB para una Vita
con HENkaku/taiHEN. Es un agente de kernel, no un shell: hoy sólo permite
`ping` e `info`. No ejecuta texto como comandos, no instala VPK, no modifica
archivos y no expone una consola remota.

## Transportes

- **Wi-Fi TCP, puerto 39999:** canal principal para la red local. Cada mensaje
  lleva HMAC-SHA-256 con una clave de 32 bytes compilada por el propietario.
  Una solicitud con clave errónea se descarta sin respuesta.
- **USB serial:** diagnóstico local opcional. Puede competir con VitaShell USB
  o Content Manager; el servicio Wi-Fi sigue iniciando aunque USB no pueda
  obtener el controlador.

La autenticación evita que otros equipos de la red usen el agente. Mantén la
Vita en una red privada: la primera versión no contiene operaciones de escritura
ni secretos, y cualquier futura operación administrativa requerirá confirmación
visible en la Vita y un protocolo con desafío anti-repetición.

## Compilar

Genera una clave nueva que no se publique ni se añada a Git:

```powershell
$pairingKey = [Convert]::ToHexString([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32))
```

Compila el plugin con esa misma clave:

```powershell
docker run --rm -v "${PWD}:/work" -w /work opennow-build sh -lc `
  "cmake -S vita-adb/kernel -B vita-adb/build -DVITA_ADBD_PAIRING_KEY_HEX=$pairingKey && cmake --build vita-adb/build"
```

El resultado es `vita-adb/build/vita-adb.skprx`. No lo instales todavía si no
has guardado una copia de `ur0:tai/config.txt`. Para activarlo manualmente,
copia el archivo a `ur0:tai/vita-adbd.skprx`, añade esa ruta bajo `*KERNEL` y
reinicia. Para deshacerlo, elimina la línea y el archivo, y reinicia.

## PC

Con la Vita y el PC en la misma red, exporta la clave sólo para la sesión actual:

```powershell
$env:VITA_ADBD_KEY = $pairingKey
py vita-adb/host/vita_adb.py --host 192.168.18.32 ping
py vita-adb/host/vita_adb.py --host 192.168.18.32 info
```

Para USB, instala `pyserial` y usa el puerto COM que Windows asigne a `PS Vita
Type D`:

```powershell
py -m pip install -r vita-adb/host/requirements.txt
py vita-adb/host/vita_adb.py devices
py vita-adb/host/vita_adb.py --port COM7 ping
```

## Verificación incluida

`py vita-adb/host/test_vita_adb.py` prueba las tramas, firma y detección de
alteraciones. `kernel/tests/sha256_test.c` valida el vector estándar HMAC-SHA-256
antes de generar el plugin.
