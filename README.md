# AlbumCard

Next generation of audio taggers

## Features

- Edit ID3v2.4.0 frames
- Edit tags of multiple files thanks to grouping as album
- [x] Generate filenames from tags
- [ ] Generate playlist files
- [ ] Support for many music filetypes (MP3, OGG, FLAC, WMA, WAV, and more)

### Rename Pattern Tokens

Here's list of all supported `%{}` tokens used for renaming

| Token        | Tag representation                                     |
| ------------ | ------------------------------------------------------ |
| `%{artists}` | All track's artist separated by `;`.                   |
| `%{artist}`  | The first track's artist.                              |
| `%{title}`   | The track's title.                                     |
| `%{album}`   | The album's title.                                     |
| `%{year}`    | The track's release year.                              |
| `%{aartist}` | The first album's artist.                              |
| `%{track}`   | The track's position in album (or disk if applicable). |
| `%{disk}`    | The track's disk position in album.                    |

## Installation

Binary downloads are available on the [release page](https://github.com/pisekpiskovec/AlbumCard/releases/latest).

### From Source

```bash
git clone https://github.com/pisekpiskovec/AlbumCard.git
just install
```

## Support and Issue Reporting

Please report all bugs and feature requests in the [issue tracker](https://github.com/pisekpiskovec/AlbumCard/issues).
