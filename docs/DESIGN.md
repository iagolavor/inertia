# Inertia — Design Philosophy

## Essência

Inertia é uma rede social **local-first** e **efémera** para o seu círculo próximo. O visual deve ser **limpo, minimalista e familiar** — no sentido básico do Instagram: fotos em destaque, perfil pessoal, feed cronológico — mas **sem** algoritmos, anúncios ou doomscrolling.

---

## Princípios visuais

### 1. Menos é mais
- Muito espaço em branco (ou superfície escura no dark mode).
- Uma ação principal por ecrã.
- Tipografia simples, sem ornamentos.
- Bordas suaves (8–12px), sombras mínimas ou inexistentes.

### 2. Conteúdo em primeiro lugar
- Fotos e posts ocupam o centro da atenção.
- Metadados (tempo restante, estado de entrega) são discretos.
- Avatares identificam pessoas; o identicon é fallback, não protagonista.

### 3. Familiar, não copiado
- Grelha de fotos no perfil (como referência visual, não como clone).
- Feed cronológico de posts dos amigos.
- Sem stories, reels, likes públicos ou contadores de seguidores.

### 4. Honestidade sobre o estado
- Indicador **online / offline** sempre visível junto ao ponto de estado.
- Entregas falhadas visíveis no outbox — transparência, não esconder erros.
- Conteúdo efémero: mostrar quando um post expira.

---

## Paleta e tema

| Token | Uso |
|-------|-----|
| `--bg` | Fundo principal |
| `--surface` | Cartões, nav, painéis |
| `--text` | Texto principal |
| `--muted` | Legendas, metadados, labels de estado |
| `--accent` | Links e ações primárias |
| `--success` | Online, entregue |
| `--danger` | Offline, falha |

Suporte a **dark** e **light** mode. O utilizador escolhe; o sistema não impõe.

---

## Componentes-chave

### Status (online / offline)
Ponto colorido + label textual `online` ou `offline` lado a lado. Sem ambiguidade.

### Perfil
- Cabeçalho: avatar, nome, estado.
- **Fotos pessoais**: grelha local, guardadas no dispositivo.
- **Posts**: histórico do utilizador no perfil; cada post também entra no feed dos amigos.

### Post
- Texto opcional + foto opcional.
- Autor, tempo relativo, tempo até expirar (48h).
- Visual de cartão simples, sem engagement chrome.

### Feed (homepage)
- Cronológico, só amigos (contactos P2P).
- O utilizador publica → post fica no perfil local → é enviado aos amigos subscritos (contactos).
- Mensagem efémera: desaparece após 48 horas.

---

## O que evitar

- Feeds infinitos optimizados para retenção.
- Notificações agressivas ou badges de "novidades".
- UI densa com muitos botões e tabs.
- Gradientes, glassmorphism ou trends visuais passageiros.
- Qualquer elemento que sugira escala massiva (seguidores, viralidade).

---

## Tom de voz (UI copy)

- Direto e calmo.
- Português ou inglês conforme contexto do utilizador; sem jargão técnico na superfície.
- Explicar P2P e efemeridade só quando necessário (onboarding, erros).

---

## Relação com a visão técnica

Este documento complementa [VISION.md](./VISION.md):

| Conceito | Design | Técnica |
|----------|--------|---------|
| Perfil | Grelha de fotos + posts | SQLite + blobs locais |
| Post | Cartão no feed | `ContentType::Post`, TTL 48h |
| Feed | Homepage cronológica | `local_posts` + inbox de amigos |
| Amigos | Círculo fechado | Contactos P2P = "subscritores" do perfil |

---

## Referência rápida de layout

```
┌─────────────────────────────┐
│  Nav · tema · avatar ● online│
├─────────────────────────────┤
│  Feed                       │
│  ┌─────────────────────┐    │
│  │ @nome · há 2h · 46h │    │
│  │ [foto opcional]     │    │
│  │ texto do post       │    │
│  └─────────────────────┘    │
│  ...                        │
├─────────────────────────────┤
│  Perfil                     │
│  [avatar] Nome ● online     │
│  ┌───┬───┬───┐              │
│  │ 📷│ 📷│ 📷│  fotos       │
│  └───┴───┴───┘              │
│  Posts · Novo post          │
└─────────────────────────────┘
```
