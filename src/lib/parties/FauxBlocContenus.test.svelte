<script lang="ts">
  // Pour les tests du lot S3 : un bloc des contenus qui n'a rien (comme un jeu sans mise à jour, DLC ni mod).
  let { vide = $bindable(false) }: { id?: number; titre?: string; plateforme?: string; deplie?: boolean; vide?: boolean } = $props();
  vide = true;
</script>
