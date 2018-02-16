/// Implementation of a level 3 book.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::cell::RefCell;
use std::rc::Rc;
use price::Price;

#[derive(Copy, Clone, Debug)]
pub enum Side {
    Bid,
    Ask
}

pub struct PriceLevel {
    orders: VecDeque<Rc<RefCell<Order>>>,
    total_size: Price,
    open_size: Price,
}

impl PriceLevel {
    fn new() -> PriceLevel {
        PriceLevel{
            orders: VecDeque::new(),
            total_size: Price::zero(),
            open_size: Price::zero()
        }
    }

    pub fn total_size(&self) -> Price { self.total_size }
    pub fn open_size(&self) -> Price { self.open_size }

    fn on_add(&mut self, ord: Rc<RefCell<Order>>) {
        self.total_size += ord.borrow().orig_size;
        self.orders.push_back(ord);
    }

    fn on_open(&mut self, size: Price) {
        assert!(size >= Price::zero());
        self.open_size += size;
    }

    fn on_match_maker(&mut self, size: Price) {
        //self.total_size -= size;
        self.open_size -= size;
        assert!(self.open_size >= Price::zero());
    }

    fn on_change(&mut self, delta: Price) {
        self.open_size += delta;
        assert!(self.open_size >= Price::zero());
    }

    fn on_done(&mut self, size: Price) {
        assert!(size >= Price::zero());
        self.open_size -= size;
        assert!(self.open_size >= Price::zero());
    }
}

pub trait Level2EventListener {
    //fn on_new_level(side: Side, price: Price) -> LevelState;
    //fn on_remove_level(side: Side, price: Price, level_state: LevelState);
    fn on_level_change(side: Side, price: Price, old_size: Price, new_size: Price, /*level_state: &mut LevelState*/);
}

type OrdersByPrice = BTreeMap<OrderPrice, PriceLevel>;

pub struct Order {
    id: String,
    side: Side,
    px: OrderPrice,
    orig_size: Price,
    open_size: Price,
}

impl Order {
    pub fn new(id: String, side: Side, px: OrderPrice, size: Price) -> Order {
        Order{id, side, px, orig_size: size, open_size: size}
    }

    fn on_open(&mut self, remaining_size: Price) {
        assert!(remaining_size >= Price::zero());
        self.open_size = remaining_size;
    }

    fn on_match_maker(&mut self, size: Price) {
        self.open_size -= size;
        assert!(self.open_size >= Price::zero());
    }

    fn on_match_taker(&mut self, size: Price) {
    }

    fn on_change(&mut self, delta: Price) {
        self.open_size += delta;
        assert!(self.open_size >= Price::zero());
    }

    fn on_done(&mut self, reason: DoneReason) -> Price {
        if reason == DoneReason::Filled {
            assert_eq!(Price::zero(), self.open_size);
        }
        self.open_size
    }
}

impl<'a> From<&'a NewOrderEvent<'a>> for Order {
    fn from(o: &NewOrderEvent<'a>) -> Self {
        Order{
            id: o.order_id.to_owned(),
            side: o.side,
            px: o.px,
            orig_size: o.orig_size,
            open_size: o.open_size,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Time(u64);
#[derive(Copy, Clone, Debug)]
pub struct Sequence(u64);

/// The type and price of the order.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum OrderPrice {
    Market,
    Limit(Price),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DoneReason {
    Filled,
    Canceled,
}

// Event structs. These include the data that is common to all feeds.

pub struct NewOrderEvent<'a> {
    seq: Sequence,
    time: Time,
    order_id: &'a str,
    side: Side,
    px: OrderPrice,
    orig_size: Price,
    open_size: Price,
}

pub struct OpenEvent<'a> {
    seq: Sequence,
    time: Time,
    order_id: &'a str,
    remaining_size: Price,
}

pub struct MatchEvent<'a> {
    seq: Sequence,
    time: Time,
    maker_order_id: &'a str,
    taker_order_id: &'a str,
    side: Side,
    price: Price,
    size: Price,
}

pub struct ChangeEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    price: OrderPrice,
    old_size_or_funds: Price,
    new_size_or_funds: Price,
}

pub struct DoneEvent<'a> {
    time: Time,
    seq: Sequence,
    order_id: &'a str,
    reason: DoneReason,
}

pub trait Level3FeedListener {
    fn on_add<'a>(&mut self, order: &NewOrderEvent<'a>);
    fn on_open<'a>(&mut self, event: &OpenEvent<'a>);
    fn on_match<'a>(&mut self, event: &MatchEvent<'a>);
    fn on_change<'a>(&mut self, event: &ChangeEvent<'a>);
    fn on_done<'a>(&mut self, event: &DoneEvent<'a>);
}

pub struct Book {
    bid: OrdersByPrice,
    ask: OrdersByPrice,
    orders: HashMap<String, Rc<RefCell<Order>>>
}

impl Book {
    pub fn new() -> Book {
        Book{ bid: OrdersByPrice::new(), ask: OrdersByPrice::new(), orders: HashMap::new() }
    }

    pub fn price_level(&self, side: Side, px: Price) -> Option<&PriceLevel> {
        // Market order "levels" are not currently exposed.
        match side {
            Side::Bid => self.bid.get(&OrderPrice::Limit(px)),
            Side::Ask => self.ask.get(&OrderPrice::Limit(px)),
        }
    }

    fn price_level_mut(&mut self, side: Side, px: OrderPrice) -> Option<&mut PriceLevel> {
        match side {
            Side::Bid => self.bid.get_mut(&px),
            Side::Ask => self.ask.get_mut(&px),
        }
    }
}

impl Level3FeedListener for Book {
    fn on_add<'a>(&mut self, event: &NewOrderEvent<'a>) {
        let ord = Rc::new(RefCell::new(Order::from(event)));
        let entry = {
            self.orders.insert(event.order_id.to_owned(), ord.clone());
            match &event.side {
                &Side::Bid => self.bid.entry(event.px),
                &Side::Ask => self.ask.entry(event.px),
            }
        };
        entry.or_insert(PriceLevel::new()).on_add(ord);
    }

    fn on_open<'a>(&mut self, event: &OpenEvent<'a>) {
        let (side, px) = {
            let mut order = self.orders.get(event.order_id).expect("Unknown order ID").borrow_mut();
            order.on_open(event.remaining_size);
            (order.side, order.px)
        };

        self.price_level_mut(side, px)
            .expect("Price level with order doesn't exist!")
            .on_open(event.remaining_size);
    }

    fn on_match<'a>(&mut self, event: &MatchEvent<'a>) {
        let (maker_side, px) = {
            let mut maker = self.orders.get(event.maker_order_id).expect("Unknown order ID").borrow_mut();
            maker.on_match_maker(event.size);
            (maker.side, maker.px)
        };
        // Currently, this doesn't do anything.
        // self.orders.get(event.taker_order_id).expect("Unknown order ID").borrow_mut()
        //     .on_match_taker(event.size);
        self.price_level_mut(maker_side, px).expect("Price level with order doesn't exist!")
            .on_match_maker(event.size);
    }

    fn on_change<'a>(&mut self, event: &ChangeEvent<'a>) {
        let delta = event.new_size_or_funds - event.old_size_or_funds;
        assert!(delta <= Price::zero());
        let (side, px) = {
            let mut order = self.orders.get(event.order_id).expect("Unknown order ID").borrow_mut();
            order.on_change(delta);
            (order.side, order.px)
        };
        self.price_level_mut(side, px).expect("Price level with order doesn't exist!")
            .on_change(delta);
    }

    fn on_done<'a>(&mut self, event: &DoneEvent<'a>) {
        let (side, px, size) = {
            let mut order = self.orders.get(event.order_id).expect("Unknown order ID").borrow_mut();
            let size = order.on_done(event.reason);
            (order.side, order.px, size)
        };
        self.price_level_mut(side, px).expect("Price level with order doesn't exist!")
            .on_done(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use self::OrderPrice::{Limit, Market};

    fn px(p: f64) -> Price { Price::from(p) }

    fn new_event(order_id: &str, side: Side, px: OrderPrice, orig_size: Price) -> NewOrderEvent {
        NewOrderEvent{ seq: Sequence(0), time: Time(0), order_id, side, px, orig_size,
                       open_size: orig_size }
    }

    fn open_event(order_id: &str, remaining_size: Price) -> OpenEvent {
        OpenEvent{ seq: Sequence(0), time: Time(0), order_id, remaining_size }
    }

    fn match_event<'a>(maker_order_id: &'a str,
                       taker_order_id: &'a str,
                       side: Side,
                       price: Price,
                       size: Price) -> MatchEvent<'a> {
        MatchEvent{ seq: Sequence(0), time: Time(0), maker_order_id, taker_order_id,
                    side, price, size }
    }

    fn change_event(order_id: &str, price: OrderPrice, old_size_or_funds: Price,
                    new_size_or_funds: Price) -> ChangeEvent {
        ChangeEvent{ seq: Sequence(0), time: Time(0), order_id, price, old_size_or_funds,
                     new_size_or_funds }
    }

    fn done_event(order_id: &str, reason: DoneReason) -> DoneEvent {
        DoneEvent{ seq: Sequence(0), time: Time(0), order_id, reason }
    }

    #[test]
    fn total_size() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, Limit(px(10.00)), px(100.)));
        assert_eq!(px(100.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        book.on_add(&new_event(&"order2", Side::Bid, Limit(px(10.00)), px(90.)));
        book.on_add(&new_event(&"order3", Side::Ask, Limit(px(10.01)), px(90.)));
        assert_eq!(px(190.), book.price_level(Side::Bid, px(10.00)).unwrap().total_size());
        assert_eq!(px(90.), book.price_level(Side::Ask, px(10.01)).unwrap().total_size());
    }

    #[test]
    fn open_size() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, Limit(px(10.00)), px(100.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_add(&new_event(&"order2", Side::Bid, Limit(px(10.00)), px(90.)));
        assert_eq!(Price::zero(), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
        book.on_open(&open_event(&"order2", px(90.)));
        assert_eq!(px(90.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }

    #[test]
    fn test_match() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, Limit(px(10.00)), px(100.)));
        book.on_open(&open_event(&"order1", px(100.)));
        book.on_add(&new_event(&"order2", Side::Ask, Limit(px( 9.90)), px(40.)));
        book.on_match(&match_event(&"order1", &"order2", Side::Bid, px( 9.99), px(40.)));
        assert_eq!(px(60.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }

    #[test]
    fn market_match() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Bid, Limit(px(10.00)), px(100.)));
        book.on_open(&open_event(&"order1", px(100.)));
        book.on_add(&new_event(&"order2", Side::Ask, Market, px(40.)));
        book.on_match(&match_event(&"order1", &"order2", Side::Bid, px( 9.99), px(40.)));
        assert_eq!(px(60.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }

    #[test]
    fn change() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order1", Side::Ask, Limit(px(10.00)), px(100.)));
        book.on_open(&open_event(&"order1", px(100.)));
        book.on_change(&change_event(&"order1", Limit(px(10.00)), px(100.), px(40.)));
        assert_eq!(px(40.), book.price_level(Side::Ask, px(10.00)).unwrap().open_size());
    }

    #[test]
    fn interacting_orders() {
        let mut book = Book::new();
        book.on_add(&new_event(&"order3", Side::Ask, Limit(px( 9.99)), px(50.)));
        book.on_add(&new_event(&"order1", Side::Bid, Limit(px(10.00)), px(100.)));
        book.on_add(&new_event(&"order2", Side::Bid, Limit(px(10.00)), px(90.)));

        book.on_open(&open_event(&"order3", px(50.)));
        book.on_open(&open_event(&"order1", px(100.)));
        assert_eq!(px(50.), book.price_level(Side::Ask, px( 9.99)).unwrap().open_size());
        assert_eq!(px(100.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());

        // orders 2 and 3 interact via a hidden limit on order 2
        book.on_match(&match_event(&"order3", &"order2", Side::Ask, px( 9.99), px(50.)));
        assert_eq!(px(0.), book.price_level(Side::Ask, px( 9.99)).unwrap().open_size());
        book.on_done(&done_event(&"order3", DoneReason::Filled));
        assert_eq!(px(0.), book.price_level(Side::Ask, px( 9.99)).unwrap().open_size());

        book.on_open(&open_event(&"order2", px(40.)));
        assert_eq!(px(140.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());

        // order 2 cancels its remaining qty
        book.on_done(&done_event(&"order2", DoneReason::Canceled));
        assert_eq!(px(100.), book.price_level(Side::Bid, px(10.00)).unwrap().open_size());
    }

}
