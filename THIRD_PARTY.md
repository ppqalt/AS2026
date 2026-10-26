# Ulkopuoliset kirjastot

AS2026:n [BSD-2-Clause-lisenssi](LICENSE) koskee projektin omaa koodia, dokumentaatiota ja synteettisiä esimerkkejä. Se ei muuta riippuvuuksien lisenssejä. Kirjastojen versiot lukitaan Cargo.lock-tiedostossa; niiden omat lisenssi- ja tekijätiedot säilyvät kyseisissä Cargo-paketeissa. Poikkeuksena repositorio sisältää alla kuvatun pienen Slintin Fluent-komponenttien paikallisen sovituksen.

## Slint

GUI käyttää Slint 1.18.1:aa. Linuxin ja Windowsin työpöytäsovelluksen Slint-komponenteille valitaan [Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0](https://slint.dev/agreements/slint-royalty-free-license.pdf). Tämä ei muuta AS2026:n omaa BSD-2-Clause-lisenssiä.

Lisenssi edellyttää Slint-mainintaa jaettavassa sovelluksessa tai sen julkisella verkkosivulla. AS2026 näyttää Slintin alkuperäisen AboutSlint-komponentin sivuvalikon **Tietoja**-ikkunassa. Komponenttia ja pääsyä siihen on säilytettävä tämän Slint-lisenssin mukaista GUI-ohjelmaa jaettaessa.

### Paikallinen Fluent-sovitus

`ui/fluent` sisältää AS2026:n käyttämät Slint 1.18.1:n Fluent-komponentit ja niiden välttämättömät riippuvuudet paketista `i-slint-compiler`. Näiden lähteiden ja kuvakkeiden lisenssit säilyvät Slintin omina; AS2026:n työpöytäsovellukseen integroidussa sovituksessa käytetään samaa edellä valittua Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0 -lisenssiä. Tämä ei vaihda projektin omaa lisenssiä GPL:ksi eikä muuta Slint-lähteitä BSD-lisensoiduiksi.

Sovitus käyttää peittäviä teemapintoja ilman varjoja ja liukuvärejä. Valinnaiset animaatiot ovat yhden, oletuksena pois olevan käyttäjäasetuksen takana. Värit on sidottu yhteiseen vaaleaan, tummaan ja Dark Spruce -teemaan ja tekstikentän valikko on suomennettu. Slintin tunnuskuvat säilyvät muuttamattomina. Lähdeversio, muutokset ja päivitysohjeet ovat tiedostossa `ui/fluent/README.md`; alkuperäisten tiedostojen tiivisteet ovat tiedostossa `ui/fluent/UPSTREAM.sha256`.

Alkuperäiset tekijämaininnat on säilytetty lähteissä:

- Copyright © SixtyFPS GmbH <info@slint.dev>
- Copyright © 2025 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>, author Nathan Collins <nathan.collins@kdab.com>

Alkuperäiset Slint-lisenssivaihtoehdot ja tunnuskuvien CC-BY-ND-4.0-lisenssi ovat hakemistossa `ui/fluent/licenses`. Julkaisupaketin olemassa oleva lisenssikerääjä sisällyttää samat tekstit Slint 1.18.1:n Cargo-pakettien `LICENSES`-hakemistoista hakemistoon `dependency-licenses`; tämä tekijä- ja muutosmaininta kulkee mukana tiedostossa `THIRD_PARTY.md`.

Slintin riippuvuuksilla ja muilla Cargo-kirjastoilla on omat lisenssinsä. Kun valmiille ohjelmalle tehdään jakelupaketti, siihen liitetään myös mukana jaettavien riippuvuuksien tarvitsemat lisenssitekstit ja tekijämaininnat. Tämän repositorion BSD-2-Clause-lisenssi ei ole lupa poistaa niitä. CLI voidaan kääntää ilman Slintiä ja muita GUI-riippuvuuksia.

BSD-2-Clause:n virallinen teksti: [Open Source Initiative](https://opensource.org/license/bsd-2-clause).


## Embedded PDF font: DejaVu Sans

The PDF exporter embeds DejaVu Sans. Original license and notices:

```text
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: DejaVu fonts
Upstream-Author: Stepan Roh <src@users.sourceforge.net> (original author),
                  see /usr/share/doc/fonts-dejavu-core/AUTHORS for full list
Source: https://dejavu-fonts.github.io/

Files: *
Copyright: Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved.
 Bitstream Vera is a trademark of Bitstream, Inc.
 DejaVu changes are in public domain.
License: bitstream-vera
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of the fonts accompanying this license ("Fonts") and associated
 documentation files (the "Font Software"), to reproduce and distribute the
 Font Software, including without limitation the rights to use, copy, merge,
 publish, distribute, and/or sell copies of the Font Software, and to permit
 persons to whom the Font Software is furnished to do so, subject to the
 following conditions:
 .
 The above copyright and trademark notices and this permission notice shall
 be included in all copies of one or more of the Font Software typefaces.
 .
 The Font Software may be modified, altered, or added to, and in particular
 the designs of glyphs or characters in the Fonts may be modified and
 additional glyphs or characters may be added to the Fonts, only if the fonts
 are renamed to names not containing either the words "Bitstream" or the word
 "Vera".
 .
 This License becomes null and void to the extent applicable to Fonts or Font
 Software that has been modified and is distributed under the "Bitstream
 Vera" names.
 .
 The Font Software may be sold as part of a larger software package but no
 copy of one or more of the Font Software typefaces may be sold by itself.
 .
 THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
 OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT,
 TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL BITSTREAM OR THE GNOME
 FOUNDATION BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING
 ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL DAMAGES,
 WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF
 THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM OTHER DEALINGS IN THE
 FONT SOFTWARE.
 .
 Except as contained in this notice, the names of Gnome, the Gnome
 Foundation, and Bitstream Inc., shall not be used in advertising or
 otherwise to promote the sale, use or other dealings in this Font Software
 without prior written authorization from the Gnome Foundation or Bitstream
 Inc., respectively. For further information, contact: fonts at gnome dot
 org.

Files: debian/*
Copyright: (C) 2005-2006 Peter Cernak <pce@users.sourceforge.net>
           (C) 2006-2011 Davide Viti <zinosat@tiscali.it>
           (C) 2011-2013 Christian Perrier <bubulle@debian.org>
           (C) 2013 Fabian Greffrath <fabian+debian@greffrath.com>
License: GPL-2+
 This program is free software; you can redistribute it
 and/or modify it under the terms of the GNU General Public
 License as published by the Free Software Foundation; either
 version 2 of the License, or (at your option) any later
 version.
 .
 This program is distributed in the hope that it will be
 useful, but WITHOUT ANY WARRANTY; without even the implied
 warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
 PURPOSE.  See the GNU General Public License for more
 details.
 .
 You should have received a copy of the GNU General Public
 License along with this package; if not, write to the Free
 Software Foundation, Inc., 51 Franklin St, Fifth Floor,
 Boston, MA  02110-1301 USA
 .
 On Debian systems, the full text of the GNU General Public
 License version 2 can be found in the file
 /usr/share/common-licenses/GPL-2'.

```
