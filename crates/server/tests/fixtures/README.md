# Images synthétiques

`pixel.png` est une image RGBA de 1 × 1 pixel, couleur `(36, 135, 102, 255)`.
Elle est générée localement avec les modules standard `struct` et `zlib` de
Python : signature PNG, IHDR RGBA 8 bits, ligne avec filtre 0 compressée et
IEND, chaque chunk portant son CRC. Elle ne contient aucune donnée utilisateur.

SHA-256 : `b3d95a3a27774ebba4d8f7a7210228b9ea845ad6bb78367e333c806ba007a645`.

Les tests l’utilisent pour les signatures, les droits d’accès, le stockage
objet, les plages d’octets et la restauration historique. Ils ne prétendent pas
certifier tous les décodeurs d’images ou le rendu visuel du navigateur.
