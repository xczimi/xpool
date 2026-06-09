import { useI18n } from '../i18n/useI18n'

/** Where data-rights requests go. Matches the result-user / admin address. */
const CONTACT_EMAIL = 'pool@xczimi.com'

/** Last substantive revision of the policy copy below. */
const LAST_UPDATED = '9 June 2026'
const LAST_UPDATED_HU = '2026. június 9.'

/**
 * Privacy policy (`.scratch/privacy-policy-and-compliance`). A plain-language,
 * self-authored policy for a friends-and-family pool — honest about the data we
 * hold and who processes it. Bilingual: the body switches on `locale` (the prose
 * is too long for the string catalogue; only the title + footer link are keyed).
 * Deliberately minimal — no Terms of Service, no self-serve deletion UI, no
 * cookie banner (we set only Auth0's functional session cookies).
 */
export function PrivacyPage() {
  const { t, locale } = useI18n()
  return (
    <section className="page">
      <h2>{t('privacyTitle')}</h2>
      {locale === 'hu' ? <HungarianBody /> : <EnglishBody />}
    </section>
  )
}

function EnglishBody() {
  return (
    <>
      <p>
        xPool is a small, free, invite-only soccer prediction pool run by a
        private individual for friends. This page explains what personal data we
        hold, why, and the choices you have.
      </p>

      <h3>What we collect and why</h3>
      <ul>
        <li>
          <strong>Your identity</strong> — email address, a nickname, and a full
          name. We use these to sign you in and to show who made which
          prediction.
        </li>
        <li>
          <strong>Your predictions</strong> — the scores and standings you
          enter. These are the point of the game: we store them to score the
          pool and show the leaderboard.
        </li>
        <li>
          <strong>Who invited you</strong> — when you accept an invite, we record
          which member's link you used, so pools know their members.
        </li>
      </ul>
      <p>
        We do <strong>not</strong> run analytics or advertising trackers, and we
        do not sell or share your data for marketing.
      </p>

      <h3>Where it is stored</h3>
      <p>
        Your data lives in an Amazon DynamoDB database hosted on Amazon Web
        Services in Canada (ca-central-1). The site and its API run on AWS.
      </p>

      <h3>Who processes it</h3>
      <ul>
        <li>
          <strong>Auth0 (Okta)</strong> — handles sign-in and stores your login
          identity.
        </li>
        <li>
          <strong>Google</strong> — only if you choose to sign in with a Google
          account, as an identity provider.
        </li>
        <li>
          <strong>Amazon Web Services</strong> — hosting and the database.
        </li>
        <li>
          <strong>Amazon SES (AWS)</strong> — sends invite and notification
          emails.
        </li>
      </ul>

      <h3>Cookies</h3>
      <p>
        We use only the functional cookies Auth0 needs to keep you signed in.
        There are no advertising or analytics cookies, so there is no cookie
        consent banner.
      </p>

      <h3>How long we keep it</h3>
      <p>
        We keep your account and predictions for the life of the tournament and a
        reasonable period afterwards so results stay viewable. Ask us to delete
        your data sooner and we will.
      </p>

      <h3>Your rights</h3>
      <p>
        You can ask to see, correct, or delete the personal data we hold about
        you, including your account and predictions. There is no self-service
        delete button yet — email{' '}
        <a href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a> and we will handle
        it. Deletion removes your login identity, your player record, and your
        predictions.
      </p>

      <h3>Contact</h3>
      <p>
        Questions or requests:{' '}
        <a href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a>.
      </p>

      <p>
        <small>Last updated: {LAST_UPDATED}.</small>
      </p>
    </>
  )
}

function HungarianBody() {
  return (
    <>
      <p>
        Az xPool egy kicsi, ingyenes, csak meghívóval elérhető focitippjáték,
        amelyet egy magánszemély üzemeltet a barátainak. Ez az oldal elmondja,
        milyen személyes adatokat kezelünk, miért, és milyen jogaid vannak.
      </p>

      <h3>Mit gyűjtünk és miért</h3>
      <ul>
        <li>
          <strong>Az azonosságod</strong> — e-mail-cím, becenév és teljes név.
          Ezeket a bejelentkezéshez használjuk, és hogy látszódjon, ki melyik
          tippet adta.
        </li>
        <li>
          <strong>A tippjeid</strong> — a beírt eredmények és tabellák. Ez maga a
          játék lényege: eltároljuk, hogy pontozzuk a tutit és mutassuk a
          ranglistát.
        </li>
        <li>
          <strong>Ki hívott meg</strong> — amikor elfogadsz egy meghívót,
          rögzítjük, melyik tag linkjével léptél be, hogy a tutik ismerjék a
          tagjaikat.
        </li>
      </ul>
      <p>
        <strong>Nem</strong> használunk analitikai vagy hirdetési követőket, és
        nem adjuk el, illetve nem osztjuk meg az adataidat marketing célból.
      </p>

      <h3>Hol tároljuk</h3>
      <p>
        Az adataid egy Amazon DynamoDB adatbázisban vannak, amelyet az Amazon Web
        Services üzemeltet Kanadában (ca-central-1). Az oldal és az API az AWS-en
        fut.
      </p>

      <h3>Ki dolgozza fel</h3>
      <ul>
        <li>
          <strong>Auth0 (Okta)</strong> — a bejelentkezést kezeli, és tárolja a
          belépési azonosságodat.
        </li>
        <li>
          <strong>Google</strong> — csak ha Google-fiókkal lépsz be, mint
          azonosságszolgáltató.
        </li>
        <li>
          <strong>Amazon Web Services</strong> — a tárhely és az adatbázis.
        </li>
        <li>
          <strong>Amazon SES (AWS)</strong> — a meghívó- és értesítő e-maileket
          küldi.
        </li>
      </ul>

      <h3>Sütik</h3>
      <p>
        Csak azokat a működéshez szükséges sütiket használjuk, amelyek az Auth0
        bejelentkezésedet fenntartják. Nincs hirdetési vagy analitikai süti, ezért
        nincs sütibeleegyezési sáv sem.
      </p>

      <h3>Meddig őrizzük</h3>
      <p>
        A fiókodat és a tippjeidet a torna idejére és egy ésszerű ideig azután is
        megőrizzük, hogy az eredmények láthatók maradjanak. Ha kéred, hamarabb is
        töröljük az adataidat.
      </p>

      <h3>A jogaid</h3>
      <p>
        Kérheted a rólad tárolt személyes adatok — köztük a fiókod és a tippjeid —
        megtekintését, helyesbítését vagy törlését. Önkiszolgáló törlés gomb még
        nincs: írj a{' '}
        <a href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a> címre, és
        elintézzük. A törlés eltávolítja a belépési azonosságodat, a
        játékosrekordodat és a tippjeidet.
      </p>

      <h3>Kapcsolat</h3>
      <p>
        Kérdés vagy kérés:{' '}
        <a href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a>.
      </p>

      <p>
        <small>Utolsó frissítés: {LAST_UPDATED_HU}</small>
      </p>
    </>
  )
}
