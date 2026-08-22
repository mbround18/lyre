CREATE OR REPLACE FUNCTION notify_queue_update()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        PERFORM pg_notify('queue_updates', OLD.guild_id);
        RETURN OLD;
    ELSE
        PERFORM pg_notify('queue_updates', NEW.guild_id);
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER current_queue_notify_trigger
AFTER INSERT OR UPDATE OR DELETE ON current_queue
FOR EACH ROW
EXECUTE FUNCTION notify_queue_update();
