# OpenNOW Vita

[![Compilar VPK](https://github.com/zero-phoenix/OpenNOW-vita/actions/workflows/build.yml/badge.svg)](https://github.com/zero-phoenix/OpenNOW-vita/actions/workflows/build.yml)
[![Última versión](https://img.shields.io/github/v/release/zero-phoenix/OpenNOW-vita?label=release)](https://github.com/zero-phoenix/OpenNOW-vita/releases/latest)
[![Licencia MPL-2.0](https://img.shields.io/badge/licencia-MPL--2.0-blue.svg)](LICENSE)

Cliente homebrew no oficial de **GeForce NOW para PlayStation Vita**. Inicia sesión en la consola,
presenta tu biblioteca y reproduce el stream con WebRTC y decodificación H.264 por hardware.

> No está afiliado a NVIDIA ni a GeForce NOW. Necesitas tu propia cuenta de GeForce NOW.

## Descargar e instalar

1. Abre [Releases](https://github.com/zero-phoenix/OpenNOW-vita/releases/latest).
2. Descarga `opennow-vita.vpk` de la versión más reciente.
3. Instálalo con VitaShell o con el instalador de aplicaciones de Vita3K.
4. Abre **OpenNOW Vita**, inicia sesión mediante el código mostrado y selecciona un juego.

Cada release contiene exactamente el VPK construido por GitHub Actions para la etiqueta publicada.

## Qué incluye

- Inicio de sesión de NVIDIA por código de dispositivo.
- Biblioteca de juegos, búsqueda, favoritos y selección de variantes.
- Sesiones reales de GeForce NOW mediante WebRTC, H.264 y audio Opus.
- Perfil **Juego** para el mando y perfil **Escritorio** para usar la Vita como ratón/teclado.
- Barra durante el stream: salida, estadísticas, teclado, trackpad y controles.
- Idiomas: español, inglés, francés y ruso.
- Preferencias locales de región, bitrate, controles y visualización.

## Alcance deliberado

OpenNOW Vita es sólo el cliente de streaming. No incorpora FTP persistente, control general de la
consola, capturas automáticas, subida de archivos ni envío de telemetría a GitHub.

El trabajo independiente de diagnóstico, administración limitada y desarrollo para el sistema
completo de PS Vita está en [Vita-System-Lab](https://github.com/zero-phoenix/Vita-System-Lab).
No es una dependencia de OpenNOW ni se ejecuta dentro de esta aplicación.

## Compatibilidad y pruebas

| Entorno | Qué se valida |
|---|---|
| PS Vita real | Inicio de sesión, red, reproducción y controles. |
| Vita3K | Arranque, interfaz, instalación VPK y mando. Su red de Vita no permite validar la sesión de GeForce NOW. |
| GitHub Actions | 73 pruebas de `opennow-core` y compilación reproducible del VPK. |

## Compilar localmente

Se necesita Docker. El mismo contenedor utilizado localmente se usa para validar el VPK:

```powershell
docker build -t opennow-build -f scripts/Dockerfile.build .
docker run --rm -v "${PWD}:/work" -w /work opennow-build sh scripts/docker-build.sh vpk
```

El archivo generado es:

```text
target/armv7-sony-vita-newlibeabihf/release/opennow-vita.vpk
```

## Problemas frecuentes

- **Vita3K no inicia sesión:** su red `sceNet` no basta para autenticar contra NVIDIA. Úsalo para
  validar interfaz e instalación; usa una Vita real para streaming.
- **Límite de dispositivos:** cierra sesiones antiguas de GeForce NOW y vuelve a intentar.
- **Imagen o audio inestable:** consulta las estadísticas durante el stream y prueba una región
  distinta o un bitrate menor.

Los logs locales están en `ux0:data/opennow/logs/`.

## Desarrollo y publicación

Los cambios se prueban con `cargo test -p opennow-core --target x86_64-unknown-linux-gnu` y con
la compilación del VPK. Al publicar una etiqueta `v*`, GitHub Actions crea automáticamente una
release con el VPK adjunto.

Consulta [CHANGELOG.md](CHANGELOG.md) para el historial y [LICENSE](LICENSE) para la licencia.
