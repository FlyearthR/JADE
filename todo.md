# XP

- [ ] Faire des XPs

    - [ ]  sur le PFV

        - [ ] lister les syscalls des implems QUIC

    - [ ] juste sur une communication d'une implem de QUIC

    - [ ] Faire des scénarios de test pour vérifier le déterminisme et voir s'il faut prober

    - [ ] Utiliser le simulateur dans QUIC interop runner

- [ ] Tester avec MiniP

- [ ] Étendre des tests en IPv6

# GROS TRAVAIL

- [ ] Gérer le routage des packets

- [ ] Gérer le multithreading

- [ ] Faire un CLI (//Mininet) (//API)

- [ ] Faire une image d'un moment pour pouvoir reprendre la simulation à partir de ce point (//RR)

- [ ] Génerer des CEX exécutables pour pouvoir les exécuter en dehors du simulateur (//RR)
 
# PETIT TRAVAIL

- [x] Rendre la topo dynamique

- [x] Gérer le random

- [x] Ajouter des infos de perturbation de lien (jitter, perte, ...)

- [x] Seeder une simulation pour que =/= seed donnent =/= résultats

- [x] Faire un système de logs propres

- [ ] Avoir une API utilisable par Ivy (sur le temps, l'état de la topo, ...)

- [ ] Fixer le bug des queues qui ne supportent pas le execve

- [ ] Supporter qu'un nœud soit multi-process

- [ ] Divulguer les PIDs pour pouvoir se greffer dessus avec GDB (et voir si ça a des implications)

- [ ] Changer le nom des interfaces pour récupérer le nom du gml

- [ ] Vérifier création fichier log + # nodes

- [ ] Ajouter wireshark
 
# C

- [ ] Fixer tous les TODOs

- [ ] Supporter les messages couche IP
 
# CODE EXTERNE

- [ ] Mettre la création de topo dans DµNE et DµNE dans NTS

- [ ] Supporter une façon plus efficace de décrire une topo avec quels exe run où (passer sur DµNE ?)

- [ ] S'interfacer avec MPF

- [ ] S'interfacer avec mininet

- [ ] Voir ce qu'on peut faire avec RR
 