-- A payment QR alongside the typed account details.
--
-- Alipay and WeChat are paid by scanning a code, not by typing a number — an account field
-- alone means the operator asks for a screenshot over chat and pastes it somewhere outside
-- the system. Attaching it to the request keeps "who asked, for how much, and where to send
-- it" in one row.
--
-- Same contract as `users.avatar`: a base64 `data:` URL of a raster image, validated by
-- `clean_avatar` before it is written, so SVG (which can carry script) cannot land here and
-- a remote URL that would never render cannot either.
--
-- Nullable, because typing an account number is still the normal case for a bank transfer
-- or PayPal, and requiring an image there would be asking for a picture of a sort code.
--
-- This is payout data with a person's payment identity in it. It belongs to the account
-- that submitted it and to an admin, and to nobody else.
ALTER TABLE withdrawals
  ADD COLUMN IF NOT EXISTS qr TEXT;
